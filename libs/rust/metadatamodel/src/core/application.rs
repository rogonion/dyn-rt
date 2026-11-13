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
/// the metadata model. Errors when there are more wildcards than indices.
pub fn schema_path(template: &str, indices: &[usize]) -> Result<String, MetadatamodelError> {
    let segments = parse_template(template)?;
    let mut indices = indices.iter();
    let mut out = RecursiveDescentSegments(Vec::with_capacity(segments.0.len()));
    for group in &segments.0 {
        let mut new_group = RecursiveDescentSegment(Vec::with_capacity(group.0.len()));
        for segment in &group.0 {
            let selector = match &segment.selector {
                PathSelector::Wildcard => {
                    let index =
                        indices
                            .next()
                            .ok_or_else(|| MetadatamodelError::PathTemplateInvalid {
                                message: format!(
                                    "template {template:?} has more [*] than the {} indices given",
                                    indices.len()
                                ),
                            })?;
                    PathSelector::Index(*index as isize)
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
/// trimmed instead of indexed (single values have no index). Errors when a
/// non-trailing wildcard has no index left.
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
                    let index =
                        indices
                            .next()
                            .ok_or_else(|| MetadatamodelError::PathTemplateInvalid {
                                message: format!(
                                    "template {template:?} needs more indices for its [*] wildcards"
                                ),
                            })?;
                    new_group.0.push(PathSegment {
                        access_strategy: AccessStrategy::Positional,
                        selector: PathSelector::Index(*index as isize),
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
