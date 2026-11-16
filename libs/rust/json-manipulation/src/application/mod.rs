//! Application use cases: `get` / `set` / `delete` over reflected data at a
//! parsed JSONPath (port of go-json's `object` package onto dyn-rt reflect).
//!
//! Shared traversal helpers live here; the three use cases are in
//! `get.rs` / `set.rs` / `delete.rs`.

mod are_equal;
mod delete;
mod for_each;
mod get;
mod object;
mod set;

pub use are_equal::{CustomEqualFn, are_equal, are_equal_with};
pub use delete::delete;
pub use for_each::for_each;
pub use get::get;
pub use object::{DynObject, PathInput, Source};
pub use set::set;

use std::any::TypeId;

use dyn_rt_json_path::{PathSegment, PathSelector, RecursiveDescentSegments, SliceSelector};
use dyn_rt_json_schema::{SchemaNode, convert_node};
use dyn_rt_reflect::{Reflect, ReflectRef, ReflectRefMut, ReflectStruct, TypeInfo};
use dyn_rt_shared::Value;

use crate::error::ManipulationError;

// ---------------------------------------------------------------------------
// indirection unwrapping
// ---------------------------------------------------------------------------

/// Unwraps one level of indirection: the inner of `Box`/`Rc`/... (via
/// [`dyn_rt_reflect::ReflectIndirection::reflect_inner_ref`]), or the payload
/// of an `Option::Some` (enums route to the enum shape). Returns `None` for
/// non-wrappers and for empty wrappers (`None`).
pub(crate) fn unwrap(value: &dyn Reflect) -> Option<&dyn Reflect> {
    match value.reflect_ref() {
        ReflectRef::Indirection(indirection) => indirection.reflect_inner_ref(),
        ReflectRef::Enum(enumeration) => {
            if enumeration.reflect_active_variant() == "Some" {
                enumeration.reflect_active_variant_payload()
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Mutable variant of [`unwrap`].
pub(crate) fn unwrap_mut(value: &mut dyn Reflect) -> Option<&mut dyn Reflect> {
    match value.reflect_ref_mut() {
        ReflectRefMut::Indirection(indirection) => indirection.reflect_inner_mut(),
        ReflectRefMut::Enum(enumeration) => {
            if enumeration.reflect_active_variant() == "Some" {
                enumeration.reflect_active_variant_payload_mut()
            } else {
                None
            }
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// path helpers
// ---------------------------------------------------------------------------

/// Resolves a (possibly negative) path index against a sequence length.
/// Negative indices count from the end (`-1` = last).
pub(crate) fn resolve_index(index: isize, len: usize) -> Result<usize, ManipulationError> {
    if index >= 0 {
        let idx = index as usize;
        if idx >= len {
            return Err(ManipulationError::ValueAtPathSegmentInvalid {
                message: format!("index {index} out of range (length {len})"),
            });
        }
        Ok(idx)
    } else {
        let idx = len as isize + index;
        if idx < 0 {
            return Err(ManipulationError::ValueAtPathSegmentInvalid {
                message: format!("index {index} out of range (length {len})"),
            });
        }
        Ok(idx as usize)
    }
}

/// Like [`resolve_index`], but a non-negative index at or past the end is
/// allowed (set expands the sequence); only negative-out-of-range errors.
pub(crate) fn resolve_set_index(index: isize, len: usize) -> Result<usize, ManipulationError> {
    if index >= 0 {
        return Ok(index as usize);
    }
    resolve_index(index, len)
}

/// Expands a slice selector `[start:end:step]` into element indices, mirroring
/// go's bounds behavior (an explicit out-of-range bound is an error; a
/// non-positive step behaves as step 1).
/// Field of a struct addressed by a selector: keys by name; non-negative
/// indices by positional field name ("0", "1", ...) — tuples are
/// struct-shaped with positional names, so `$.1` addresses the second tuple
/// element.
pub(crate) fn struct_field_by_selector<'a>(
    structure: &'a dyn ReflectStruct,
    selector: &PathSelector,
) -> Option<&'a dyn Reflect> {
    match selector {
        PathSelector::Key(key) => structure.reflect_field(key),
        PathSelector::Index(index) if *index >= 0 => structure.reflect_field(&index.to_string()),
        _ => None,
    }
}

/// Mutable variant of [`struct_field_by_selector`].
pub(crate) fn struct_field_by_selector_mut<'a>(
    structure: &'a mut dyn ReflectStruct,
    selector: &PathSelector,
) -> Option<&'a mut dyn Reflect> {
    match selector {
        PathSelector::Key(key) => structure.reflect_field_mut(key),
        PathSelector::Index(index) if *index >= 0 => {
            structure.reflect_field_mut(&index.to_string())
        }
        _ => None,
    }
}

/// Type info of the struct field addressed by a selector (see
/// [`struct_field_by_selector`]).
pub(crate) fn struct_field_type_info_by_selector(
    structure: &dyn ReflectStruct,
    selector: &PathSelector,
) -> Option<&'static TypeInfo> {
    match selector {
        PathSelector::Key(key) => structure.reflect_field_type_info(key),
        PathSelector::Index(index) if *index >= 0 => {
            structure.reflect_field_type_info(&index.to_string())
        }
        _ => None,
    }
}

/// Resolves the sequence-targeting members of a union selector into concrete
/// indices: index members (negatives resolved against `len`), slice members
/// expanded via [`slice_indices`], wildcard members expand to all indices.
/// Key members are not sequence-targeting and are ignored here (the map and
/// struct union arms handle those).
pub(crate) fn union_member_indices(
    members: &[PathSegment],
    len: usize,
) -> Result<Vec<usize>, ManipulationError> {
    let mut indices = Vec::new();
    for member in members {
        match &member.selector {
            PathSelector::Index(index) => {
                if let Ok(idx) = resolve_index(*index, len) {
                    indices.push(idx);
                }
            }
            PathSelector::Slice(slice) => {
                indices.extend(slice_indices(slice, len)?);
            }
            PathSelector::Wildcard => indices.extend(0..len),
            _ => {}
        }
    }
    Ok(indices)
}

pub(crate) fn slice_indices(
    slice: &SliceSelector,
    len: usize,
) -> Result<Vec<usize>, ManipulationError> {
    let resolve_bound = |bound: isize, is_start: bool| -> Result<usize, ManipulationError> {
        if bound >= 0 {
            let v = bound as usize;
            // go errors when an explicit start/end is >= length
            if v >= len {
                return Err(ManipulationError::PathSegmentInvalid {
                    message: format!(
                        "slice bound {bound} out of range (length {len}){}",
                        if is_start { " (start)" } else { " (end)" }
                    ),
                });
            }
            Ok(v)
        } else {
            let v = len as isize + bound;
            if v < 0 {
                return Err(ManipulationError::PathSegmentInvalid {
                    message: format!("slice bound {bound} out of range (length {len})"),
                });
            }
            Ok(v as usize)
        }
    };

    let start = match slice.start {
        Some(s) => resolve_bound(s, true)?,
        None => 0,
    };
    let step = match slice.step {
        Some(s) if s > 0 => {
            let v = s as usize;
            if v >= len {
                return Err(ManipulationError::PathSegmentInvalid {
                    message: format!("slice step {s} out of range (length {len})"),
                });
            }
            v
        }
        _ => 1,
    };
    let end = match slice.end {
        Some(e) => resolve_bound(e, false)?,
        None => len,
    };

    let mut indices = Vec::new();
    let mut i = start;
    while i < end {
        if i < len {
            indices.push(i);
        }
        i += step;
    }
    Ok(indices)
}

/// A scalar schema node for a runtime-known scalar type. `TypeInfo`s built by
/// reflect's `type_info()` carry no constructor, so besides `scalar_of` the
/// known scalar types are matched explicitly by `TypeId`.
pub(crate) fn scalar_node_for(type_info: &'static TypeInfo) -> Option<SchemaNode> {
    if let Some(node) = SchemaNode::scalar_of(type_info) {
        return Some(node);
    }
    let id = type_info.type_id;
    if id == TypeId::of::<String>() {
        Some(SchemaNode::scalar::<String>())
    } else if id == TypeId::of::<i64>() {
        Some(SchemaNode::scalar::<i64>())
    } else if id == TypeId::of::<f64>() {
        Some(SchemaNode::scalar::<f64>())
    } else if id == TypeId::of::<bool>() {
        Some(SchemaNode::scalar::<bool>())
    } else {
        None
    }
}

/// A fresh instance of a runtime-known type: the carried constructor when
/// present, else the zero of the known scalar types / `Value::Null`.
pub(crate) fn construct_value(type_info: &TypeInfo) -> Option<Box<dyn Reflect>> {
    if let Some(instance) = type_info.construct() {
        return Some(instance);
    }
    let id = type_info.type_id;
    if id == TypeId::of::<String>() {
        Some(Box::new(String::new()))
    } else if id == TypeId::of::<i64>() {
        Some(Box::new(0i64))
    } else if id == TypeId::of::<f64>() {
        Some(Box::new(0f64))
    } else if id == TypeId::of::<bool>() {
        Some(Box::new(false))
    } else if id == TypeId::of::<Value>() {
        Some(Box::new(Value::Null))
    } else {
        None
    }
}

/// Converts a path key string into a map's key type via schema conversion
/// (port of go's `mapKey` conversion with a scalar node of the key type).
pub(crate) fn convert_path_key(
    key: &str,
    key_type_info: &'static TypeInfo,
) -> Result<Box<dyn Reflect>, ManipulationError> {
    // string keys are the common case (Value objects, map[string]...): direct
    if key_type_info.type_id == std::any::TypeId::of::<String>() {
        return Ok(Box::new(key.to_string()));
    }
    let node =
        scalar_node_for(key_type_info).ok_or_else(|| ManipulationError::ConversionFailed {
            message: format!(
                "no constructor for map key type {}",
                key_type_info.type_name
            ),
        })?;
    let mut target =
        construct_value(key_type_info).ok_or_else(|| ManipulationError::ConversionFailed {
            message: format!(
                "no constructor for map key type {}",
                key_type_info.type_name
            ),
        })?;
    convert_node(&Value::from(key.to_string()), &node, &mut *target).map_err(|err| {
        ManipulationError::ConversionFailed {
            message: format!(
                "convert map key {key:?} to type {} failed: {err}",
                key_type_info.type_name
            ),
        }
    })?;
    Ok(target)
}

/// Whether a reflected map key equals a path key string (keys render via
/// their scalar string form, like go's `mapKeyString`).
pub(crate) fn key_matches(reflected_key: &dyn Reflect, key: &str) -> bool {
    reflected_key.reflect_scalar_string().as_deref() == Some(key)
}

/// The child values of a container (map values, sequence elements, struct
/// fields) — used by the wildcard selector.
pub(crate) fn children(value: &dyn Reflect) -> Result<Vec<&dyn Reflect>, ManipulationError> {
    match value.reflect_ref() {
        ReflectRef::Map(map) => Ok(map.reflect_iter().map(|(_, v)| v).collect()),
        ReflectRef::Sequence(sequence) => Ok(sequence.reflect_iter().collect()),
        ReflectRef::Struct(structure) => {
            let count = structure.reflect_field_count();
            let mut out = Vec::with_capacity(count);
            for index in 0..count {
                if let Some(field) = structure.reflect_field_at(index) {
                    out.push(field);
                }
            }
            Ok(out)
        }
        other => Err(ManipulationError::PathSegmentInvalid {
            message: format!(
                "wildcard selector unsupported for kind {}",
                kind_label(&other)
            ),
        }),
    }
}

pub(crate) fn kind_label(value: &ReflectRef<'_>) -> &'static str {
    match value {
        ReflectRef::Scalar(_) => "scalar",
        ReflectRef::Sequence(_) => "sequence",
        ReflectRef::Map(_) => "map",
        ReflectRef::Struct(_) => "struct",
        ReflectRef::Enum(_) => "enum",
        ReflectRef::Indirection(_) => "indirection",
        ReflectRef::Unknown(_) => "unknown",
    }
}

pub(crate) fn kind_label_mut(value: &ReflectRefMut<'_>) -> &'static str {
    match value {
        ReflectRefMut::Scalar(_) => "scalar",
        ReflectRefMut::Sequence(_) => "sequence",
        ReflectRefMut::Map(_) => "map",
        ReflectRefMut::Struct(_) => "struct",
        ReflectRefMut::Enum(_) => "enum",
        ReflectRefMut::Indirection(_) => "indirection",
        ReflectRefMut::Unknown(_) => "unknown",
    }
}

/// The selector's label for error messages.
pub(crate) fn selector_label(selector: &PathSelector) -> String {
    match selector {
        PathSelector::Unrecognized => "<unrecognized>".to_string(),
        PathSelector::Root => "$".to_string(),
        PathSelector::Wildcard => "*".to_string(),
        PathSelector::Key(key) => format!("key {key:?}"),
        PathSelector::Index(index) => format!("index [{index}]"),
        PathSelector::Slice(slice) => format!("slice {slice:?}"),
        PathSelector::Union(members) => format!(
            "union [{}]",
            members
                .iter()
                .map(|m| selector_label(&m.selector))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// Whether the segments hold anything at all (a lone `$` still has a root
/// segment; a completely empty parse is treated as the root).
pub(crate) fn is_root_only(segments: &RecursiveDescentSegments) -> bool {
    segments.0.is_empty()
        || (segments.0.len() == 1 && segments.0[0].0.is_empty())
        || (segments.0.len() == 1
            && segments.0[0].0.len() == 1
            && matches!(segments.0[0].0[0].selector, PathSelector::Root))
}
