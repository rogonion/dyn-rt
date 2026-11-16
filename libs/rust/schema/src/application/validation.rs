//! Validation use cases: check reflected data against a schema (port of
//! go-json's validation.go). Dispatches on the schema node's shape and walks
//! the data through the reflect shape traits (`as_struct` / `as_map` / ...),
//! so both typed values and `shared::Value` work as data.

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

use dyn_rt_reflect::{
    Reflect, ReflectRef, as_enum, as_map, as_sequence, as_struct, kind_of, reflect_with_inner,
};
use dyn_rt_shared::Value;

use crate::domain::{
    Schema, SchemaNode, SchemaPathSegment, SchemaSet, SchemaShape, SchemaValidator, render_path,
};
use crate::error::SchemaError;

// ---------------------------------------------------------------------------
// validation
// ---------------------------------------------------------------------------

/// Options for validation (port of go's `Validation` struct).
pub struct ValidationOptions {
    /// Stop at the first matching node of a `Schema::Set` (go's default).
    pub validate_on_first_match: bool,
    /// Custom validators keyed by the data's `TypeId` (go's `Validators`).
    pub custom_validators: HashMap<TypeId, Arc<dyn SchemaValidator>>,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        ValidationOptions {
            validate_on_first_match: true,
            custom_validators: HashMap::new(),
        }
    }
}

impl ValidationOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_validate_on_first_match(mut self, value: bool) -> Self {
        self.validate_on_first_match = value;
        self
    }

    pub fn with_custom_validator(
        mut self,
        type_id: TypeId,
        validator: Arc<dyn SchemaValidator>,
    ) -> Self {
        self.custom_validators.insert(type_id, validator);
        self
    }
}

/// The outcome of a successful validation: which `Schema::Set` keys matched
/// (go's `ValidSchemaNodeKeys`, returned instead of mutating the schema).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ValidationOutcome {
    pub valid_set_keys: Vec<String>,
}

/// Validates `data` against `schema` with default options.
pub fn validate(data: &dyn Reflect, schema: &Schema) -> Result<ValidationOutcome, SchemaError> {
    validate_with(data, schema, &ValidationOptions::default())
}

/// Validates `data` against `schema` with explicit options.
pub fn validate_with(
    data: &dyn Reflect,
    schema: &Schema,
    options: &ValidationOptions,
) -> Result<ValidationOutcome, SchemaError> {
    let mut outcome = ValidationOutcome::default();
    validate_schema(data, schema, &[], options, &mut outcome)?;
    Ok(outcome)
}

/// Validates `data` against a single schema node with default options.
pub fn validate_node(data: &dyn Reflect, node: &SchemaNode) -> Result<(), SchemaError> {
    validate_node_with(
        data,
        node,
        &[],
        &ValidationOptions::default(),
        &mut ValidationOutcome::default(),
    )
}

fn validate_schema(
    data: &dyn Reflect,
    schema: &Schema,
    path: &[SchemaPathSegment],
    options: &ValidationOptions,
    outcome: &mut ValidationOutcome,
) -> Result<(), SchemaError> {
    match schema {
        Schema::Set(set) => validate_set(data, set, path, options, outcome),
        Schema::Node(node) => validate_node_with(data, node, path, options, outcome),
    }
}

fn validate_node_with(
    data: &dyn Reflect,
    node: &SchemaNode,
    path: &[SchemaPathSegment],
    options: &ValidationOptions,
    outcome: &mut ValidationOutcome,
) -> Result<(), SchemaError> {
    let rendered = render_path(path);
    let fail = |message: String| SchemaError::ValidationFailed {
        message,
        path: rendered.clone(),
    };

    if is_nil(data) {
        if node.nilable {
            return Ok(());
        }
        return Err(fail("data cannot be nil".to_string()));
    }

    if matches!(node.shape, SchemaShape::Any) {
        return Ok(());
    }

    if kind_of(data) != node.kind() {
        return Err(fail(format!(
            "data kind {:?} is not valid (expected {:?})",
            kind_of(data),
            node.kind()
        )));
    }

    if let Some(validator) = &node.validator {
        return validator.validate(data, node, path);
    }

    if let Some(validator) = options
        .custom_validators
        .get(&data.reflect_as_any().type_id())
    {
        return validator.validate(data, node, path);
    }

    match &node.shape {
        SchemaShape::Scalar => {
            let data_type_id = data.reflect_as_any().type_id();
            let data_name = data.reflect_type_name();
            let node_name = node.type_info.type_name;
            // Name match: exact, or on the short (last `::` segment) form,
            // case-insensitive — so a `json_scalar("string")` node matches
            // both `Value::String` ("string") and `String` ("alloc::string::String").
            let name_match = data_name == node_name
                || short_type_name(data_name).eq_ignore_ascii_case(short_type_name(node_name));
            if node.type_info.type_id == TypeId::of::<Value>() {
                // A json-scalar node (Value variant name) matches by name only;
                // every Value shares Value's TypeId, so TypeId alone cannot
                // distinguish variants.
                if name_match {
                    return Ok(());
                }
            } else if data_type_id == node.type_info.type_id || name_match {
                return Ok(());
            }
            Err(fail(format!(
                "data type '{data_name}' is not valid (expected '{node_name}')"
            )))
        }
        SchemaShape::Any => Ok(()),
        SchemaShape::Indirection { inner } => reflect_with_inner(data, |inner_value| {
            validate_schema(inner_value, inner, path, options, outcome)
        })
        .map_err(|err| fail(format!("indirection has no inner value: {err}")))?,
        SchemaShape::Sequence { elements, items } => {
            let elements = elements.as_ref().ok_or_else(|| {
                fail("schema to validate element(s) in data (sequence) not found".to_string())
            })?;
            let sequence =
                as_sequence(data).ok_or_else(|| fail("data is not a sequence".to_string()))?;
            for (index, element) in sequence.reflect_iter().enumerate() {
                let mut child_path = path.to_vec();
                child_path.push(SchemaPathSegment::Index(index));
                let child_schema = items
                    .iter()
                    .find(|(item_index, _)| *item_index == index)
                    .map(|(_, schema)| schema)
                    .unwrap_or(elements);
                validate_schema(element, child_schema, &child_path, options, outcome)?;
            }
            Ok(())
        }
        SchemaShape::Map {
            entries,
            key,
            value,
        } => {
            if entries.is_empty() && (key.is_none() || value.is_none()) {
                return Err(fail(
                    "no schema to validate entries in data (map) found".to_string(),
                ));
            }
            let map = as_map(data).ok_or_else(|| fail("data is not a map".to_string()))?;
            let mut validated: Vec<String> = Vec::new();
            for (map_key, map_value) in map.reflect_iter() {
                let key_string = key_to_string(map_key);
                let mut child_path = path.to_vec();
                child_path.push(SchemaPathSegment::Key(
                    key_string
                        .clone()
                        .unwrap_or_else(|| "<non-string>".to_string()),
                ));

                if let Some(key_string) = &key_string {
                    if let Some((_, entry_schema)) = entries.iter().find(|(k, _)| k == key_string) {
                        let key_schema = match entry_schema {
                            Schema::Node(entry_node) => entry_node
                                .entry_key_schema
                                .as_ref()
                                .unwrap_or(key.as_ref().expect("checked above")),
                            _ => key.as_ref().expect("checked above"),
                        };
                        validate_schema(map_key, key_schema, &child_path, options, outcome)?;
                        validate_schema(map_value, entry_schema, &child_path, options, outcome)?;
                        validated.push(key_string.clone());
                        continue;
                    }
                }

                match (key, value) {
                    (Some(key_schema), Some(value_schema)) => {
                        validate_schema(map_key, key_schema, &child_path, options, outcome)?;
                        validate_schema(map_value, value_schema, &child_path, options, outcome)?;
                        if let Some(key_string) = &key_string {
                            validated.push(key_string.clone());
                        }
                    }
                    _ => {
                        return Err(fail(format!(
                            "schema for map key '{}' not found",
                            key_string.unwrap_or_else(|| "<non-string>".to_string())
                        )));
                    }
                }
            }
            if node.children_must_be_valid && validated.len() != entries.len() {
                return Err(fail(
                    "not all child nodes are present and validated against".to_string(),
                ));
            }
            Ok(())
        }
        SchemaShape::Struct { fields } => {
            if fields.is_empty() {
                return Err(fail(
                    "no schema for properties in data struct found".to_string(),
                ));
            }
            let structure =
                as_struct(data).ok_or_else(|| fail("data is not a struct".to_string()))?;
            let mut validated: Vec<&'static str> = Vec::new();
            for (field_name, field_value) in structure.reflect_iter_fields() {
                let Some((_, field_schema)) = fields.iter().find(|(name, _)| name == field_name)
                else {
                    continue;
                };
                let mut child_path = path.to_vec();
                child_path.push(SchemaPathSegment::Key(field_name.to_string()));
                validate_schema(field_value, field_schema, &child_path, options, outcome)?;
                validated.push(field_name);
            }
            if node.children_must_be_valid && validated.len() != fields.len() {
                return Err(fail(
                    "not all child nodes are present and validated against".to_string(),
                ));
            }
            Ok(())
        }
        SchemaShape::Enum { variants } => {
            let enumeration =
                as_enum(data).ok_or_else(|| fail("data is not an enum".to_string()))?;
            let active = enumeration.reflect_active_variant();
            let Some((_, variant_schema)) = variants.iter().find(|(name, _)| name == active) else {
                return Err(fail(format!("no schema for active variant '{active}'")));
            };
            match variant_schema {
                None => Ok(()),
                Some(variant_schema) => {
                    if let Some(payload) = enumeration.reflect_active_variant_payload() {
                        validate_schema(payload, variant_schema, path, options, outcome)?;
                    } else {
                        // Struct variant: validate each inline field against
                        // the variant schema.
                        for index in 0..enumeration.reflect_active_variant_field_infos().len() {
                            let field = enumeration
                                .reflect_active_variant_field_at(index)
                                .expect("index in bounds");
                            validate_schema(field, variant_schema, path, options, outcome)?;
                        }
                    }
                    Ok(())
                }
            }
        }
    }
}

fn validate_set(
    data: &dyn Reflect,
    set: &SchemaSet,
    path: &[SchemaPathSegment],
    options: &ValidationOptions,
    outcome: &mut ValidationOutcome,
) -> Result<(), SchemaError> {
    if set.nodes.is_empty() {
        return Err(SchemaError::ValidationFailed {
            message: "no schema nodes found".to_string(),
            path: render_path(path),
        });
    }

    let mut matched: Vec<String> = Vec::new();
    let mut last_err: Option<SchemaError> = None;

    let mut try_node = |key: &str,
                        node: &SchemaNode,
                        matched: &mut Vec<String>,
                        last_err: &mut Option<SchemaError>|
     -> Result<bool, SchemaError> {
        match validate_node_with(data, node, path, options, outcome) {
            Ok(()) => {
                matched.push(key.to_string());
                Ok(true)
            }
            Err(err) => {
                *last_err = Some(err);
                Ok(false)
            }
        }
    };

    if let Some(default_key) = &set.default_key {
        if let Some((_, node)) = set.nodes.iter().find(|(key, _)| key == default_key) {
            if try_node(default_key, node, &mut matched, &mut last_err)? {
                outcome.valid_set_keys.extend(matched);
                return Ok(());
            }
        }
    }

    for (key, node) in &set.nodes {
        if Some(key) == set.default_key.as_ref() {
            continue;
        }
        if try_node(key, node, &mut matched, &mut last_err)? && options.validate_on_first_match {
            break;
        }
    }

    if matched.is_empty() {
        return Err(last_err.unwrap_or_else(|| SchemaError::ValidationFailed {
            message: "data not valid against any schema node".to_string(),
            path: render_path(path),
        }));
    }
    outcome.valid_set_keys.extend(matched);
    Ok(())
}

/// Nil detection: an indirection without an inner value, or `Option::None`
/// (an enum whose variant table is exactly `Some`/`None` and whose active
/// variant carries no payload).
pub(crate) fn is_nil(data: &dyn Reflect) -> bool {
    if data
        .reflect_as_any()
        .downcast_ref::<Value>()
        .is_some_and(Value::is_null)
    {
        return true;
    }
    match data.reflect_ref() {
        ReflectRef::Indirection(indirection) => {
            let mut nil = false;
            indirection.reflect_get_inner(&mut |inner| {
                if inner.is_err() {
                    nil = true;
                }
            });
            nil
        }
        ReflectRef::Enum(enumeration) => {
            let names: Vec<&'static str> = enumeration
                .reflect_variants()
                .iter()
                .map(|v| v.name)
                .collect();
            names == ["Some", "None"] && enumeration.reflect_active_variant_payload().is_none()
        }
        _ => false,
    }
}

/// The last `::` segment of a type name, e.g. `alloc::string::String` -> `String`.
fn short_type_name(name: &str) -> &str {
    name.rsplit("::").next().unwrap_or(name)
}

/// String form of a map key (`String` or `Value::String`), when it is one.
pub(crate) fn key_to_string(key: &dyn Reflect) -> Option<String> {
    if let Some(string) = key.reflect_as_any().downcast_ref::<String>() {
        return Some(string.clone());
    }
    if let Some(Value::String(string)) = key.reflect_as_any().downcast_ref::<Value>() {
        return Some(string.clone());
    }
    None
}
