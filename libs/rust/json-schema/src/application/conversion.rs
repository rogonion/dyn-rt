//! Conversion use cases: convert reflected data into the shape a schema
//! dictates (port of go-json's conversion.go), driving an existing target
//! `&mut dyn Reflect` in place through the reflect mutable shape traits.
//!
//! Unlike go (which builds a new `reflect.Value` of the schema's type), the
//! Rust port converts INTO a caller-provided target — the same semantic
//! json-manipulation's `set` needs. Scalar coercions (string <-> number <->
//! bool, etc.) follow go's convertTo{Int,Uint,Float,String,Bool}.

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

use dyn_rt_reflect::{
    Reflect, as_enum_mut, as_indirection_mut, as_map, as_map_mut, as_sequence, as_sequence_mut,
    as_struct, as_struct_mut,
};
use dyn_rt_shared::{Number, Value};

use super::validation::{is_nil, key_to_string};
use crate::domain::{
    Schema, SchemaConverter, SchemaNode, SchemaPathSegment, SchemaSet, SchemaShape, render_path,
};
use crate::error::SchemaError;

/// Options for conversion (port of go's `Conversion` struct).
#[derive(Default)]
pub struct ConversionOptions {
    /// Custom converters keyed by the SOURCE data's `TypeId` (go's
    /// `Converters` map keyed by source type).
    pub custom_converters: HashMap<TypeId, Arc<dyn SchemaConverter>>,
}

impl ConversionOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_custom_converter(
        mut self,
        source_type_id: TypeId,
        converter: Arc<dyn SchemaConverter>,
    ) -> Self {
        self.custom_converters.insert(source_type_id, converter);
        self
    }
}

/// The outcome of a successful conversion: which `Schema::Set` keys matched.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ConversionOutcome {
    pub valid_set_keys: Vec<String>,
}

/// Converts `source` into `target` as dictated by `schema`, with default options.
pub fn convert(
    source: &dyn Reflect,
    schema: &Schema,
    target: &mut dyn Reflect,
) -> Result<ConversionOutcome, SchemaError> {
    convert_with(source, schema, target, &ConversionOptions::default())
}

/// Converts `source` into `target` as dictated by `schema`, with explicit options.
pub fn convert_with(
    source: &dyn Reflect,
    schema: &Schema,
    target: &mut dyn Reflect,
    options: &ConversionOptions,
) -> Result<ConversionOutcome, SchemaError> {
    let mut outcome = ConversionOutcome::default();
    convert_schema(source, schema, target, &[], options, &mut outcome)?;
    Ok(outcome)
}

/// Converts `source` into `target` as dictated by a single schema node.
pub fn convert_node(
    source: &dyn Reflect,
    node: &SchemaNode,
    target: &mut dyn Reflect,
) -> Result<(), SchemaError> {
    convert_node_with(
        source,
        node,
        target,
        &[],
        &ConversionOptions::default(),
        &mut ConversionOutcome::default(),
    )
}

fn convert_schema(
    source: &dyn Reflect,
    schema: &Schema,
    target: &mut dyn Reflect,
    path: &[SchemaPathSegment],
    options: &ConversionOptions,
    outcome: &mut ConversionOutcome,
) -> Result<(), SchemaError> {
    match schema {
        Schema::Set(set) => convert_set(source, set, target, path, options, outcome),
        Schema::Node(node) => convert_node_with(source, node, target, path, options, outcome),
    }
}

#[allow(clippy::too_many_arguments)]
fn convert_node_with(
    source: &dyn Reflect,
    node: &SchemaNode,
    target: &mut dyn Reflect,
    path: &[SchemaPathSegment],
    options: &ConversionOptions,
    outcome: &mut ConversionOutcome,
) -> Result<(), SchemaError> {
    let fail = |message: String| SchemaError::ConversionFailed {
        message,
        path: render_path(path),
    };

    if is_nil(source) {
        if node.nilable {
            // Leave the target as-is (e.g. Option stays None).
            return Ok(());
        }
        return Err(fail("source cannot be nil".to_string()));
    }

    if matches!(node.shape, SchemaShape::Any) {
        return target
            .reflect_apply(source)
            .map_err(|err| fail(format!("apply failed: {err}")));
    }

    // Custom converters run before the pass-through: an explicit converter
    // wins over direct assignment.
    if let Some(converter) = &node.converter {
        let converted = converter.convert(source, node, path)?;
        return target
            .reflect_apply(&*converted)
            .map_err(|err| fail(format!("apply of converted value failed: {err}")));
    }

    if let Some(converter) = options
        .custom_converters
        .get(&source.reflect_as_any().type_id())
    {
        let converted = converter.convert(source, node, path)?;
        return target
            .reflect_apply(&*converted)
            .map_err(|err| fail(format!("apply of converted value failed: {err}")));
    }

    // Same concrete type: assign directly (primitives, identical structs, ...).
    if source.reflect_as_any().type_id() == node.type_info.type_id {
        return target
            .reflect_apply(source)
            .map_err(|err| fail(format!("apply failed: {err}")));
    }

    match &node.shape {
        SchemaShape::Scalar => {
            let converted = convert_scalar(source, node, path)?;
            target
                .reflect_apply(&*converted)
                .map_err(|err| fail(format!("apply of scalar failed: {err}")))
        }
        SchemaShape::Any => Ok(()),
        SchemaShape::Struct { fields } => {
            let target_struct =
                as_struct_mut(target).ok_or_else(|| fail("target is not a struct".to_string()))?;
            if let Some(source_struct) = as_struct(source) {
                // struct source: convert field by field, matched by name
                for (field_name, field_schema) in fields {
                    let Some(source_field) = source_struct.reflect_field(field_name) else {
                        continue;
                    };
                    let Some(target_field) = target_struct.reflect_field_mut(field_name) else {
                        continue;
                    };
                    let mut child_path = path.to_vec();
                    child_path.push(SchemaPathSegment::Key(field_name.clone()));
                    if let Err(err) = convert_schema(
                        source_field,
                        field_schema,
                        target_field,
                        &child_path,
                        options,
                        outcome,
                    ) {
                        if !node.nilable {
                            return Err(err);
                        }
                    }
                }
                Ok(())
            } else if let Some(source_map) = as_map(source) {
                // map source (e.g. Value object): convert entry by entry
                for (map_key, map_value) in source_map.reflect_iter() {
                    let Some(key_string) = key_to_string(map_key) else {
                        return Err(fail(format!(
                            "map key '{}' is not a string for struct conversion",
                            map_key.reflect_type_name()
                        )));
                    };
                    let Some((_, field_schema)) =
                        fields.iter().find(|(name, _)| *name == key_string)
                    else {
                        return Err(fail(format!(
                            "schema for field '{key_string}' not found for struct conversion"
                        )));
                    };
                    let Some(target_field) = target_struct.reflect_field_mut(&key_string) else {
                        return Err(fail(format!(
                            "map key '{key_string}' is not a valid field in struct conversion"
                        )));
                    };
                    let mut child_path = path.to_vec();
                    child_path.push(SchemaPathSegment::Key(key_string));
                    if let Err(err) = convert_schema(
                        map_value,
                        field_schema,
                        target_field,
                        &child_path,
                        options,
                        outcome,
                    ) {
                        if !node.nilable {
                            return Err(err);
                        }
                    }
                }
                Ok(())
            } else {
                Err(fail(format!(
                    "unsupported source kind {:?} for struct conversion",
                    source.reflect_ref().kind()
                )))
            }
        }
        SchemaShape::Map {
            entries,
            key,
            value,
        } => {
            let target_map =
                as_map_mut(target).ok_or_else(|| fail("target is not a map".to_string()))?;
            // Source entries as (key string, value); struct sources contribute
            // their field names as keys.
            let mut source_entries: Vec<(String, &dyn Reflect)> = Vec::new();
            if let Some(source_map) = as_map(source) {
                for (map_key, map_value) in source_map.reflect_iter() {
                    let key_string = key_to_string(map_key).ok_or_else(|| {
                        fail("map key is not a string for map conversion".to_string())
                    })?;
                    source_entries.push((key_string, map_value));
                }
            } else if let Some(source_struct) = as_struct(source) {
                for (name, field_value) in source_struct.reflect_iter_fields() {
                    source_entries.push((name.to_string(), field_value));
                }
            } else {
                return Err(fail(format!(
                    "unsupported source kind {:?} for map conversion",
                    source.reflect_ref().kind()
                )));
            };
            for (key_string, source_value) in source_entries {
                let mut child_path = path.to_vec();
                child_path.push(SchemaPathSegment::Key(key_string.clone()));

                let value_schema = match entries.iter().find(|(k, _)| *k == key_string) {
                    Some((_, entry_schema)) => entry_schema.clone(),
                    None => match value {
                        Some(value_schema) => (**value_schema).clone(),
                        None => {
                            return Err(fail(format!(
                                "schema for map key '{key_string}' not found"
                            )));
                        }
                    },
                };
                // The source key as a reflectable string, for key conversion.
                let source_key = String::from(&key_string);
                let converted_key = match key {
                    Some(key_schema) => {
                        convert_to_boxed(&source_key, key_schema, &child_path, options, outcome)?
                    }
                    None => Box::new(source_key) as Box<dyn Reflect>,
                };
                let converted_value =
                    convert_to_boxed(source_value, &value_schema, &child_path, options, outcome)?;
                target_map
                    .reflect_set_key(&*converted_key, converted_value)
                    .map_err(|err| fail(format!("set_key failed: {err}")))?;
            }
            Ok(())
        }
        SchemaShape::Sequence { elements, items } => {
            let target_sequence = as_sequence_mut(target)
                .ok_or_else(|| fail("target is not a sequence".to_string()))?;
            let source_sequence =
                as_sequence(source).ok_or_else(|| fail("source is not a sequence".to_string()))?;
            for (index, element) in source_sequence.reflect_iter().enumerate() {
                let child_schema = items
                    .iter()
                    .find(|(item_index, _)| *item_index == index)
                    .map(|(_, schema)| schema.clone())
                    .or_else(|| elements.as_ref().map(|e| (**e).clone()))
                    .ok_or_else(|| {
                        fail("no schema to convert element(s) in sequence found".to_string())
                    })?;
                let mut child_path = path.to_vec();
                child_path.push(SchemaPathSegment::Index(index));
                let converted =
                    convert_to_boxed(element, &child_schema, &child_path, options, outcome)?;
                target_sequence
                    .reflect_push(converted)
                    .map_err(|err| fail(format!("push failed: {err}")))?;
            }
            Ok(())
        }
        SchemaShape::Indirection { inner } => {
            let target_indirection = as_indirection_mut(target)
                .ok_or_else(|| fail("target is not an indirection".to_string()))?;
            let converted = convert_to_boxed(source, inner, path, options, outcome)?;
            target_indirection
                .reflect_set_inner(converted)
                .map_err(|err| fail(format!("set_inner failed: {err}")))
        }
        SchemaShape::Enum { variants } => {
            let target_enum =
                as_enum_mut(target).ok_or_else(|| fail("target is not an enum".to_string()))?;
            if let Some(source_enum) = dyn_rt_reflect::as_enum(source) {
                // enum source: convert the active variant's payload
                let active = source_enum.reflect_active_variant();
                let Some((_, variant_schema)) = variants.iter().find(|(name, _)| name == active)
                else {
                    return Err(fail(format!("no schema for variant '{active}'")));
                };
                match variant_schema {
                    None => target_enum
                        .reflect_set_variant(active, None)
                        .map_err(|err| fail(format!("set_variant failed: {err}"))),
                    Some(variant_schema) => {
                        let payload = source_enum
                            .reflect_active_variant_payload()
                            .ok_or_else(|| fail("active variant has no payload".to_string()))?;
                        let converted =
                            convert_to_boxed(payload, variant_schema, path, options, outcome)?;
                        target_enum
                            .reflect_set_variant(active, Some(converted))
                            .map_err(|err| fail(format!("set_variant failed: {err}")))
                    }
                }
            } else if let Some((_, some_schema)) = variants.iter().find(|(name, _)| name == "Some")
            {
                // non-enum source against an Option-like node: wrap as Some
                let Some(some_schema) = some_schema else {
                    return Err(fail("variant 'Some' has no payload schema".to_string()));
                };
                let converted = convert_to_boxed(source, some_schema, path, options, outcome)?;
                target_enum
                    .reflect_set_variant("Some", Some(converted))
                    .map_err(|err| fail(format!("set_variant failed: {err}")))
            } else {
                Err(fail("source is not an enum".to_string()))
            }
        }
    }
}

fn convert_set(
    source: &dyn Reflect,
    set: &SchemaSet,
    target: &mut dyn Reflect,
    path: &[SchemaPathSegment],
    options: &ConversionOptions,
    outcome: &mut ConversionOutcome,
) -> Result<(), SchemaError> {
    if set.nodes.is_empty() {
        return Err(SchemaError::ConversionFailed {
            message: "no schema nodes found".to_string(),
            path: render_path(path),
        });
    }
    let mut last_err: Option<SchemaError> = None;

    let mut try_node = |key: &str, node: &SchemaNode| -> Option<()> {
        match convert_node_with(source, node, target, path, options, outcome) {
            Ok(()) => {
                outcome.valid_set_keys.push(key.to_string());
                Some(())
            }
            Err(err) => {
                last_err = Some(err);
                None
            }
        }
    };

    if let Some(default_key) = &set.default_key {
        if let Some((_, node)) = set.nodes.iter().find(|(key, _)| key == default_key) {
            if try_node(default_key, node).is_some() {
                return Ok(());
            }
        }
    }
    for (key, node) in &set.nodes {
        if Some(key) == set.default_key.as_ref() {
            continue;
        }
        if try_node(key, node).is_some() {
            return Ok(());
        }
    }
    Err(last_err.unwrap_or_else(|| SchemaError::ConversionFailed {
        message: "source not convertible against any schema node".to_string(),
        path: render_path(path),
    }))
}

/// Converts `source` against `schema` into a fresh boxed value: scalar nodes
/// produce the coerced scalar directly; other nodes are built in a new
/// instance from the node's `default` fn or its type's `ReflectNew`.
fn convert_to_boxed(
    source: &dyn Reflect,
    schema: &Schema,
    path: &[SchemaPathSegment],
    options: &ConversionOptions,
    outcome: &mut ConversionOutcome,
) -> Result<Box<dyn Reflect>, SchemaError> {
    match schema {
        Schema::Node(node) if matches!(node.shape, SchemaShape::Scalar) => {
            convert_scalar(source, node, path)
        }
        Schema::Node(node) => {
            let mut instance =
                new_instance_of(node).ok_or_else(|| SchemaError::ConversionFailed {
                    message: format!(
                        "no constructor (default fn or ReflectNew) for type '{}'",
                        node.type_info.type_name
                    ),
                    path: render_path(path),
                })?;
            convert_node_with(source, node, &mut *instance, path, options, outcome)?;
            Ok(instance)
        }
        Schema::Set(set) => {
            // try each node, building a fresh instance of the matching type
            let mut last_err: Option<SchemaError> = None;
            let mut try_node = |key: &str, node: &SchemaNode| -> Option<Box<dyn Reflect>> {
                let mut instance = new_instance_of(node)?;
                match convert_node_with(source, node, &mut *instance, path, options, outcome) {
                    Ok(()) => {
                        outcome.valid_set_keys.push(key.to_string());
                        Some(instance)
                    }
                    Err(err) => {
                        last_err = Some(err);
                        None
                    }
                }
            };
            if let Some(default_key) = &set.default_key {
                if let Some((_, node)) = set.nodes.iter().find(|(key, _)| key == default_key) {
                    if let Some(instance) = try_node(default_key, node) {
                        return Ok(instance);
                    }
                }
            }
            for (key, node) in &set.nodes {
                if Some(key) == set.default_key.as_ref() {
                    continue;
                }
                if let Some(instance) = try_node(key, node) {
                    return Ok(instance);
                }
            }
            Err(last_err.unwrap_or_else(|| SchemaError::ConversionFailed {
                message: "no schema nodes found".to_string(),
                path: render_path(path),
            }))
        }
    }
}

/// A fresh instance of the node's type: the node's `default` fn, else the
/// type's `ReflectNew` constructor from its `TypeInfo`.
fn new_instance_of(node: &SchemaNode) -> Option<Box<dyn Reflect>> {
    if let Some(default) = node.default {
        return Some(default());
    }
    node.type_info.new_instance.map(|new| new())
}

// ---------------------------------------------------------------------------
// scalar coercions (port of convertTo{Int,Uint,Float,String,Bool})
// ---------------------------------------------------------------------------

/// Converts a scalar source to the scalar type named by the node, boxed.
fn convert_scalar(
    source: &dyn Reflect,
    node: &SchemaNode,
    path: &[SchemaPathSegment],
) -> Result<Box<dyn Reflect>, SchemaError> {
    let fail = |message: String| SchemaError::ConversionFailed {
        message,
        path: render_path(path),
    };
    let tid = node.type_info.type_id;
    let any = source.reflect_as_any();

    macro_rules! int_target {
        ($($t:ty),*) => {
            $(if tid == TypeId::of::<$t>() {
                let value = to_i64(source).map_err(fail)?;
                return Ok(Box::new(value as $t));
            })*
        };
    }
    macro_rules! uint_target {
        ($($t:ty),*) => {
            $(if tid == TypeId::of::<$t>() {
                let value = to_u64(source).map_err(fail)?;
                return Ok(Box::new(value as $t));
            })*
        };
    }

    int_target!(i8, i16, i32, i64, isize);
    uint_target!(u8, u16, u32, u64, usize);

    if tid == TypeId::of::<f32>() || tid == TypeId::of::<f64>() {
        let value = to_f64(source).map_err(fail)?;
        return Ok(if tid == TypeId::of::<f32>() {
            Box::new(value as f32)
        } else {
            Box::new(value)
        });
    }
    if tid == TypeId::of::<bool>() {
        return Ok(Box::new(to_bool(source).map_err(fail)?));
    }
    if tid == TypeId::of::<char>() {
        let string = to_string(source).map_err(fail)?;
        let ch = string
            .chars()
            .next()
            .ok_or_else(|| fail("cannot convert empty string to char".to_string()))?;
        return Ok(Box::new(ch));
    }
    if tid == TypeId::of::<String>() {
        return Ok(Box::new(to_string(source).map_err(fail)?));
    }
    if tid == TypeId::of::<Value>() {
        // json scalar node: produce the matching Value variant
        return match node.type_info.type_name {
            "string" => Ok(Box::new(Value::String(to_string(source).map_err(fail)?))),
            "boolean" => Ok(Box::new(Value::Bool(to_bool(source).map_err(fail)?))),
            "number" => {
                let number = to_number(source).ok_or_else(|| {
                    fail(format!(
                        "cannot convert '{}' to a json number",
                        source.reflect_type_name()
                    ))
                })?;
                Ok(Box::new(Value::Number(number)))
            }
            "null" => Ok(Box::new(Value::Null)),
            other => Err(fail(format!("unsupported json scalar variant '{other}'"))),
        };
    }
    let _ = any;
    Err(fail(format!(
        "unsupported scalar target type '{}'",
        node.type_info.type_name
    )))
}

/// Source as i64 (port of convertToInt): ints, uints, floats, bools, strings.
fn to_i64(source: &dyn Reflect) -> Result<i64, String> {
    let any = source.reflect_as_any();
    macro_rules! try_int {
        ($($t:ty),*) => {
            $(if let Some(v) = any.downcast_ref::<$t>() {
                return Ok(*v as i64);
            })*
        };
    }
    try_int!(i8, i16, i32, i64, isize, u8, u16, u32);
    if let Some(v) = any.downcast_ref::<u64>() {
        return i64::try_from(*v).map_err(|_| format!("{v} overflows i64"));
    }
    if let Some(v) = any.downcast_ref::<u128>() {
        return i64::try_from(*v).map_err(|_| format!("{v} overflows i64"));
    }
    if let Some(v) = any.downcast_ref::<i128>() {
        return i64::try_from(*v).map_err(|_| format!("{v} overflows i64"));
    }
    if let Some(v) = any.downcast_ref::<f32>() {
        return Ok(*v as i64);
    }
    if let Some(v) = any.downcast_ref::<f64>() {
        return Ok(*v as i64);
    }
    if let Some(v) = any.downcast_ref::<bool>() {
        return Ok(if *v { 1 } else { 0 });
    }
    if let Some(v) = any.downcast_ref::<char>() {
        return Ok(*v as i64);
    }
    if let Some(v) = any.downcast_ref::<String>() {
        return v
            .parse::<i64>()
            .map_err(|_| format!("cannot parse '{v}' as i64"));
    }
    if let Some(Value::Number(n)) = any.downcast_ref::<Value>() {
        return Ok(n.as_f64() as i64);
    }
    if let Some(Value::String(s)) = any.downcast_ref::<Value>() {
        return s
            .parse::<i64>()
            .map_err(|_| format!("cannot parse '{s}' as i64"));
    }
    if let Some(Value::Bool(b)) = any.downcast_ref::<Value>() {
        return Ok(if *b { 1 } else { 0 });
    }
    Err(format!(
        "unsupported source type '{}' for i64 conversion",
        source.reflect_type_name()
    ))
}

/// Source as u64 (port of convertToUint).
fn to_u64(source: &dyn Reflect) -> Result<u64, String> {
    let i = to_i64(source)?;
    u64::try_from(i).map_err(|_| format!("{i} is negative for u64 conversion"))
}

/// Source as f64 (port of convertToFloat).
fn to_f64(source: &dyn Reflect) -> Result<f64, String> {
    let any = source.reflect_as_any();
    if let Some(v) = any.downcast_ref::<f64>() {
        return Ok(*v);
    }
    if let Some(v) = any.downcast_ref::<f32>() {
        return Ok(*v as f64);
    }
    if let Some(v) = any.downcast_ref::<String>() {
        return v
            .parse::<f64>()
            .map_err(|_| format!("cannot parse '{v}' as f64"));
    }
    if let Some(Value::String(s)) = any.downcast_ref::<Value>() {
        return s
            .parse::<f64>()
            .map_err(|_| format!("cannot parse '{s}' as f64"));
    }
    if let Some(Value::Number(n)) = any.downcast_ref::<Value>() {
        return Ok(n.as_f64());
    }
    Ok(to_i64(source)? as f64)
}

/// Source as bool (port of convertToBool): bool, numbers (!= 0), strings.
fn to_bool(source: &dyn Reflect) -> Result<bool, String> {
    let any = source.reflect_as_any();
    if let Some(v) = any.downcast_ref::<bool>() {
        return Ok(*v);
    }
    if let Some(v) = any.downcast_ref::<String>() {
        return v
            .parse::<bool>()
            .map_err(|_| format!("cannot parse '{v}' as bool"));
    }
    if let Some(Value::String(s)) = any.downcast_ref::<Value>() {
        return s
            .parse::<bool>()
            .map_err(|_| format!("cannot parse '{s}' as bool"));
    }
    if let Some(Value::Bool(b)) = any.downcast_ref::<Value>() {
        return Ok(*b);
    }
    Ok(to_i64(source)? != 0)
}

/// Source as String (port of convertToString): any scalar renders directly.
fn to_string(source: &dyn Reflect) -> Result<String, String> {
    let any = source.reflect_as_any();
    if let Some(v) = any.downcast_ref::<String>() {
        return Ok(v.clone());
    }
    if let Some(v) = any.downcast_ref::<bool>() {
        return Ok(v.to_string());
    }
    if let Some(v) = any.downcast_ref::<char>() {
        return Ok(v.to_string());
    }
    if let Some(v) = any.downcast_ref::<f64>() {
        return Ok(v.to_string());
    }
    if let Some(v) = any.downcast_ref::<f32>() {
        return Ok(v.to_string());
    }
    macro_rules! try_int_str {
        ($($t:ty),*) => {
            $(if let Some(v) = any.downcast_ref::<$t>() {
                return Ok(v.to_string());
            })*
        };
    }
    try_int_str!(
        i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, i128, u128
    );
    match any.downcast_ref::<Value>() {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(Value::Number(n)) => Ok(n.as_f64().to_string()),
        Some(Value::Bool(b)) => Ok(b.to_string()),
        Some(Value::Null) => Ok("null".to_string()),
        _ => Err(format!(
            "unsupported source type '{}' for string conversion",
            source.reflect_type_name()
        )),
    }
}

/// Source as a lossless json Number, when it is numeric or numeric-string.
fn to_number(source: &dyn Reflect) -> Option<Number> {
    let any = source.reflect_as_any();
    if let Some(v) = any.downcast_ref::<i64>() {
        return Some(if *v < 0 {
            Number::NegInt(*v)
        } else {
            Number::PosInt(*v as u64)
        });
    }
    if let Some(v) = any.downcast_ref::<u64>() {
        return Some(Number::PosInt(*v));
    }
    if let Some(v) = any.downcast_ref::<f64>() {
        return Some(Number::Float(*v));
    }
    if let Some(v) = any.downcast_ref::<f32>() {
        return Some(Number::Float(*v as f64));
    }
    if let Some(v) = any.downcast_ref::<String>() {
        if let Ok(i) = v.parse::<i64>() {
            return Some(if i < 0 {
                Number::NegInt(i)
            } else {
                Number::PosInt(i as u64)
            });
        }
        return v.parse::<f64>().ok().map(Number::Float);
    }
    match any.downcast_ref::<Value>() {
        Some(Value::Number(n)) => Some(*n),
        Some(Value::String(s)) => {
            if let Ok(i) = s.parse::<i64>() {
                Some(if i < 0 {
                    Number::NegInt(i)
                } else {
                    Number::PosInt(i as u64)
                })
            } else {
                s.parse::<f64>().ok().map(Number::Float)
            }
        }
        _ => None,
    }
}
