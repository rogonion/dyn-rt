//! Core application use cases (port of go-metadatamodel's `core`):
//! property getters over the JSON-like model, the templated `group:path_key`
//! post-processing (schema path vs data path), and the strict cardinality
//! check.
//!
//! Templates keep `[*]` all the way to the end (e.g.
//! `$['group:fields'][*].Bio[*]`): [`schema_path`] substitutes indices into
//! every wildcard to address the field/group props inside the model, and
//! [`data_path`] additionally strips the `group:fields[*]` structure segments
//! and resolves the remaining wildcards against the data to get the actual
//! path to data.

use dyn_rt_json_path::{
    AccessStrategy, PathSegment, PathSelector, RecursiveDescentSegment, RecursiveDescentSegments,
    RegexPathTokenizer, parse_path,
};
use dyn_rt_reflect::{Reflect, ReflectRef};
use dyn_rt_shared::Value;

use super::domain::{Cardinality, FIELDS_KEY, MetadataModel, props};
use crate::error::MetadatamodelError;

// ---------------------------------------------------------------------------
// property getters
// ---------------------------------------------------------------------------

/// Reads property `key` from a field/group object.
pub fn prop<'a>(field_group: &'a MetadataModel, key: &str) -> Option<&'a Value> {
    field_group.get(key)
}

/// The templated JSONPath of a field/group (`group:path_key`).
pub fn path_key_of(field_group: &MetadataModel) -> Result<&str, MetadatamodelError> {
    match field_group
        .get(props::GROUP_PATH_KEY)
        .and_then(Value::as_str)
    {
        Some(key) if !key.is_empty() => Ok(key),
        _ => Err(MetadatamodelError::MissingProperty {
            key: props::GROUP_PATH_KEY,
        }),
    }
}

/// Display name of a field/group: `group:name`, else the last template
/// segment, else `default` (go's GetFieldGroupName).
pub fn name_of(field_group: &MetadataModel, default: &str) -> String {
    if let Some(name) = field_group.get(props::GROUP_NAME).and_then(Value::as_str)
        && !name.is_empty()
    {
        return name.to_string();
    }
    if let Ok(key) = path_key_of(field_group) {
        let segments = parse_path(&RegexPathTokenizer, key);
        if let Some(last_group) = segments.0.last()
            && let Some(last) = last_group.0.last()
            && let PathSelector::Key(key_name) = &last.selector
        {
            return key_name.clone();
        }
    }
    if default.is_empty() {
        return "#unnamed".to_string();
    }
    default.to_string()
}

/// The fields object of a group: `group:fields` is always an array whose
/// first element is the object mapping field-name → field/group.
pub fn fields_of(field_group: &MetadataModel) -> Result<&Value, MetadatamodelError> {
    let fields = field_group
        .get(FIELDS_KEY)
        .ok_or_else(|| MetadatamodelError::InvalidModel {
            message: format!("group has no {FIELDS_KEY:?} array"),
        })?;
    let array = fields
        .as_array()
        .ok_or_else(|| MetadatamodelError::InvalidModel {
            message: format!("{FIELDS_KEY:?} is not an array"),
        })?;
    array
        .first()
        .ok_or_else(|| MetadatamodelError::InvalidModel {
            message: format!("{FIELDS_KEY:?} array is empty"),
        })
}

/// The props object of a field: the field entry is always an array whose
/// first element is the props object.
pub fn props_of(field: &Value) -> Result<&Value, MetadatamodelError> {
    let array = field
        .as_array()
        .ok_or_else(|| MetadatamodelError::InvalidModel {
            message: "field entry is not an array of props".to_string(),
        })?;
    array
        .first()
        .ok_or_else(|| MetadatamodelError::InvalidModel {
            message: "field props array is empty".to_string(),
        })
}

/// Mutable variant of [`fields_of`].
pub fn fields_of_mut(field_group: &mut MetadataModel) -> Result<&mut Value, MetadatamodelError> {
    let fields =
        field_group
            .get_mut(FIELDS_KEY)
            .ok_or_else(|| MetadatamodelError::InvalidModel {
                message: format!("group has no {FIELDS_KEY:?} array"),
            })?;
    let array = fields
        .as_array_mut()
        .ok_or_else(|| MetadatamodelError::InvalidModel {
            message: format!("{FIELDS_KEY:?} is not an array"),
        })?;
    array
        .first_mut()
        .ok_or_else(|| MetadatamodelError::InvalidModel {
            message: format!("{FIELDS_KEY:?} array is empty"),
        })
}

/// Mutable variant of [`props_of`].
pub fn props_of_mut(field: &mut Value) -> Result<&mut Value, MetadatamodelError> {
    let array = field
        .as_array_mut()
        .ok_or_else(|| MetadatamodelError::InvalidModel {
            message: "field entry is not an array of props".to_string(),
        })?;
    array
        .first_mut()
        .ok_or_else(|| MetadatamodelError::InvalidModel {
            message: "field props array is empty".to_string(),
        })
}

/// Declared cardinality of a field/group: `field:cardinality` or
/// `group:cardinality`, defaulting to [`Cardinality::Array`].
pub fn cardinality_of(field_group: &MetadataModel) -> Cardinality {
    for key in [props::FIELD_CARDINALITY, props::GROUP_CARDINALITY] {
        if let Some(value) = field_group.get(key).and_then(Value::as_str)
            && let Some(cardinality) = Cardinality::parse(value)
        {
            return cardinality;
        }
    }
    Cardinality::default()
}

/// Strict cardinality check: the value's shape must match the declaration —
/// `array` requires a sequence, `single` requires a non-sequence. No
/// dynamic exceptions.
pub fn check_value_cardinality(
    value: &dyn Reflect,
    cardinality: Cardinality,
) -> Result<(), MetadatamodelError> {
    // unwrap indirections so Box<Vec<_>>/Some<[...]> count by their inner
    let mut current = value;
    while let Some(inner) = unwrap_ref(current) {
        current = inner;
    }
    let is_sequence = matches!(current.reflect_ref(), ReflectRef::Sequence(_));
    match (cardinality, is_sequence) {
        (Cardinality::Array, true) | (Cardinality::Single, false) => Ok(()),
        (Cardinality::Array, false) => Err(MetadatamodelError::CardinalityViolation {
            message: format!(
                "declared 'array' but value is {} (single)",
                current.reflect_type_name()
            ),
        }),
        (Cardinality::Single, true) => Err(MetadatamodelError::CardinalityViolation {
            message: format!(
                "declared 'single' but value is a sequence ({})",
                current.reflect_type_name()
            ),
        }),
    }
}

/// Whether a field/group props object is a group (has a `group:fields`
/// array). go's IsFieldAGroup additionally requires read order; here the
/// fields array alone decides, and read order defaults to the fields' key
/// order (see [`read_order_of`]).
pub fn is_group(field_group: &MetadataModel) -> bool {
    field_group
        .get(FIELDS_KEY)
        .and_then(Value::as_array)
        .is_some_and(|fields| !fields.is_empty())
}

/// The read order of a group's fields: `group:read_order_of_fields` when
/// present, else the `group:fields[0]` object's key order (Rust refinement —
/// go errors when the property is absent).
pub fn read_order_of(field_group: &MetadataModel) -> Result<Vec<String>, MetadatamodelError> {
    if let Some(order) = field_group.get(props::GROUP_READ_ORDER_OF_FIELDS) {
        let array = order
            .as_array()
            .ok_or_else(|| MetadatamodelError::InvalidModel {
                message: format!("{:?} is not an array", props::GROUP_READ_ORDER_OF_FIELDS),
            })?;
        let mut out = Vec::with_capacity(array.len());
        for entry in array {
            let key = entry
                .as_str()
                .ok_or_else(|| MetadatamodelError::InvalidModel {
                    message: format!(
                        "{:?} entry is not a string",
                        props::GROUP_READ_ORDER_OF_FIELDS
                    ),
                })?;
            out.push(key.to_string());
        }
        return Ok(out);
    }
    let fields = fields_of(field_group)?;
    let object = fields
        .as_object()
        .ok_or_else(|| MetadatamodelError::InvalidModel {
            message: format!("{FIELDS_KEY}[0] is not an object"),
        })?;
    Ok(object.iter().map(|(key, _)| key.clone()).collect())
}

/// Whether any of the group's fields is itself a group (go's
/// DoesFieldGroupFieldsContainNestedGroupFields).
pub fn contains_nested_groups(field_group: &MetadataModel) -> Result<bool, MetadatamodelError> {
    let fields = fields_of(field_group)?;
    let object = fields
        .as_object()
        .ok_or_else(|| MetadatamodelError::InvalidModel {
            message: format!("{FIELDS_KEY}[0] is not an object"),
        })?;
    for (_, entry) in object {
        if let Ok(props) = props_of(entry)
            && is_group(props)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Sets property `key` on an object value, replacing any existing entry.
pub fn set_prop(object: &mut Value, key: &str, value: Value) {
    if let Some(obj) = object.as_object_mut() {
        match obj.iter_mut().find(|(k, _)| k == key) {
            Some((_, slot)) => *slot = value,
            None => obj.push((key.to_string(), value)),
        }
    }
}

/// Merges `from`'s properties into `into` (right wins), object-to-object.
/// go's MergeRightJsonObjectIntoLeft.
pub fn merge_props(into: &mut Value, from: &Value) {
    if let (Some(into_obj), Some(from_obj)) = (into.as_object_mut(), from.as_object()) {
        for (key, value) in from_obj {
            match into_obj.iter_mut().find(|(k, _)| k == key) {
                Some((_, slot)) => *slot = value.clone(),
                None => into_obj.push((key.clone(), value.clone())),
            }
        }
    }
}

fn unwrap_ref(value: &dyn Reflect) -> Option<&dyn Reflect> {
    match value.reflect_ref() {
        ReflectRef::Indirection(indirection) => indirection.reflect_inner_ref(),
        ReflectRef::Enum(enumeration) if enumeration.reflect_active_variant() == "Some" => {
            enumeration.reflect_active_variant_payload()
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// templated group:path_key post-processing
// ---------------------------------------------------------------------------

/// Parses a `group:path_key` template into segments.
pub fn parse_template(template: &str) -> Result<RecursiveDescentSegments, MetadatamodelError> {
    let segments = parse_path(&RegexPathTokenizer, template);
    if segments.0.is_empty() || segments.0.iter().all(|g| g.0.is_empty()) {
        return Err(MetadatamodelError::PathTemplateInvalid {
            message: format!("template {template:?} parsed to no segments"),
        });
    }
    Ok(segments)
}

/// Schema path: the template with every `[*]` wildcard replaced, in order,
/// by the given indices — the actual path to the field/group props inside
/// the metadata model. Missing indices default to 0 (go behavior).
pub fn schema_path(template: &str, indices: &[usize]) -> Result<String, MetadatamodelError> {
    let segments = parse_template(template)?;
    let mut indices = indices.iter();
    let mut out = RecursiveDescentSegments(Vec::with_capacity(segments.0.len()));
    for group in &segments.0 {
        let mut new_group = RecursiveDescentSegment(Vec::with_capacity(group.0.len()));
        for segment in &group.0 {
            let selector = match &segment.selector {
                PathSelector::Wildcard => {
                    // go behavior: missing indices default to 0
                    let index = indices.next().copied().unwrap_or(0);
                    PathSelector::Index(index as isize)
                }
                other => other.clone(),
            };
            new_group.0.push(PathSegment {
                access_strategy: AccessStrategy::Mixed,
                selector,
            });
        }
        out.0.push(new_group);
    }
    Ok(out.to_string())
}

/// Data path: post-processes the template into the actual path to DATA.
///
/// Every structural `group:fields[*]` (key + following wildcard) segment is
/// removed; each remaining `[*]` is replaced in order by the given indices.
/// When `cardinality` is [`Cardinality::Single`], a trailing `[*]` is
/// trimmed instead of indexed (single values have no index). Missing indices
/// default to 0 (go behavior).
pub fn data_path(
    template: &str,
    indices: &[usize],
    cardinality: Cardinality,
) -> Result<String, MetadatamodelError> {
    let segments = parse_template(template)?;
    let mut indices = indices.iter();
    let last_group_len = segments.0.last().map_or(0, |g| g.0.len());

    let mut out_groups: Vec<RecursiveDescentSegment> = Vec::new();
    for (group_index, group) in segments.0.iter().enumerate() {
        let mut new_group = RecursiveDescentSegment(Vec::new());
        let mut skip_next_wildcard = false;
        for (position, segment) in group.0.iter().enumerate() {
            if skip_next_wildcard {
                // the [*] belonging to a group:fields key: drop it too
                skip_next_wildcard = false;
                if matches!(segment.selector, PathSelector::Wildcard) {
                    continue;
                }
            }
            match &segment.selector {
                PathSelector::Key(key) if key == FIELDS_KEY => {
                    // structural schema segment: drop it and its wildcard
                    skip_next_wildcard = true;
                }
                PathSelector::Wildcard => {
                    let is_trailing_wildcard =
                        group_index == segments.0.len() - 1 && position == last_group_len - 1;
                    if is_trailing_wildcard && cardinality == Cardinality::Single {
                        continue; // single values have no index: trim the [*]
                    }
                    // go behavior: missing indices default to 0
                    let index = indices.next().copied().unwrap_or(0);
                    new_group.0.push(PathSegment {
                        access_strategy: AccessStrategy::Positional,
                        selector: PathSelector::Index(index as isize),
                    });
                }
                other => {
                    new_group.0.push(PathSegment {
                        access_strategy: other_strategy(other),
                        selector: other.clone(),
                    });
                }
            }
        }
        out_groups.push(new_group);
    }

    // drop groups left empty by the group:fields removal
    out_groups.retain(|g| !g.0.is_empty());
    if out_groups.is_empty() {
        return Ok("$".to_string());
    }
    Ok(RecursiveDescentSegments(out_groups).to_string())
}

fn other_strategy(selector: &PathSelector) -> AccessStrategy {
    match selector {
        PathSelector::Key(_) => AccessStrategy::Named,
        PathSelector::Index(_) | PathSelector::Slice(_) => AccessStrategy::Positional,
        _ => AccessStrategy::Mixed,
    }
}
