//! `get`: retrieve the value(s) at a JSONPath from a reflected source.
//!
//! Port of go-json's `object.Get` / `recursiveGet` / `selectorGetLoop` /
//! `recursiveDescentGet`. Returns every match (go returns a slice for
//! multi-match selectors and a single value otherwise — here unified as a
//! `Vec` of references into the source).

use dyn_rt_path::{PathSelector, RecursiveDescentSegments};
use dyn_rt_reflect::{Reflect, ReflectRef};

use super::{
    children, convert_path_key, is_root_only, key_matches, resolve_index, selector_label,
    slice_indices, unwrap,
};
use crate::domain::PathCursor;
use crate::error::ManipulationError;

/// Retrieves the value(s) in `source` at `segments`. An empty path or a lone
/// `$` yields the source itself. Returns one entry per match; an empty `Vec`
/// means the path simply matched nothing. Structural problems (unsupported
/// segment for the value's kind, out-of-range index/slice, nil source,
/// failed key conversion) are `Err`.
pub fn get<'a>(
    source: &'a dyn Reflect,
    segments: &RecursiveDescentSegments,
) -> Result<Vec<&'a dyn Reflect>, ManipulationError> {
    if is_root_only(segments) {
        // a lone `$` yields the source itself, unwrapping indirections so a
        // `Some(payload)` root yields the payload
        return Ok(vec![unwrap(source).unwrap_or(source)]);
    }
    let cursor = PathCursor::start(segments.0.len());
    let results = if cursor.recursive == cursor.last_recursive {
        linear_get(source, segments, &cursor)?
    } else {
        descent_get(source, segments, &cursor)?
    };
    Ok(results)
}

/// Advances past the current segment into `value` (the value the current
/// segment resolved to).
fn advance_get<'a>(
    value: &'a dyn Reflect,
    segments: &RecursiveDescentSegments,
    cursor: &PathCursor,
) -> Result<Vec<&'a dyn Reflect>, ManipulationError> {
    if cursor.is_terminal(segments) {
        Ok(vec![value])
    } else if cursor.is_group_end(segments) {
        descent_get(value, segments, &cursor.next_group())
    } else {
        linear_get(value, segments, &cursor.next_in_group())
    }
}

/// Linear traversal (no `..`): applies the current segment to `value`.
fn linear_get<'a>(
    value: &'a dyn Reflect,
    segments: &RecursiveDescentSegments,
    cursor: &PathCursor,
) -> Result<Vec<&'a dyn Reflect>, ManipulationError> {
    let segment = &segments.0[cursor.recursive].0[cursor.collection];
    let label = selector_label(&segment.selector);

    // unwrap indirections (Box/Rc/Some/...)
    if let Some(inner) = unwrap(value) {
        return linear_get(inner, segments, cursor);
    }

    match &segment.selector {
        PathSelector::Root => advance_get(value, segments, cursor),
        PathSelector::Key(key) => match value.reflect_ref() {
            ReflectRef::Map(map) => {
                let key_ref = convert_path_key(key, map.reflect_key_type_info())?;
                match map.reflect_get(&*key_ref) {
                    Some(entry) => advance_get(entry, segments, cursor),
                    None => Ok(Vec::new()),
                }
            }
            ReflectRef::Struct(structure) => match structure.reflect_field(key) {
                Some(field) => advance_get(field, segments, cursor),
                None => Ok(Vec::new()),
            },
            other => Err(ManipulationError::PathSegmentInvalid {
                message: format!(
                    "key segment {label} unsupported for kind {}",
                    super::kind_label(&other)
                ),
            }),
        },
        PathSelector::Index(index) => match value.reflect_ref() {
            ReflectRef::Sequence(sequence) => {
                let idx = resolve_index(*index, sequence.reflect_length())?;
                match sequence.reflect_get(idx) {
                    Some(element) => advance_get(element, segments, cursor),
                    None => Ok(Vec::new()),
                }
            }
            // tuples are struct-shaped with positional names: $[1] == field "1"
            ReflectRef::Struct(structure) => {
                match super::struct_field_by_selector(structure, &segment.selector) {
                    Some(field) => advance_get(field, segments, cursor),
                    None => Ok(Vec::new()),
                }
            }
            other => Err(ManipulationError::PathSegmentInvalid {
                message: format!(
                    "index segment {label} unsupported for kind {}",
                    super::kind_label(&other)
                ),
            }),
        },
        PathSelector::Wildcard => {
            let selected = children(value)?;
            selector_get(selected, segments, cursor)
        }
        PathSelector::Union(members) => {
            let selected = union_select(value, members)?;
            selector_get(selected, segments, cursor)
        }
        PathSelector::Slice(slice) => match value.reflect_ref() {
            ReflectRef::Sequence(sequence) => {
                let indices = slice_indices(slice, sequence.reflect_length())?;
                let mut selected = Vec::with_capacity(indices.len());
                for idx in indices {
                    if let Some(element) = sequence.reflect_get(idx) {
                        selected.push(element);
                    }
                }
                selector_get(selected, segments, cursor)
            }
            other => Err(ManipulationError::PathSegmentInvalid {
                message: format!(
                    "slice segment {label} unsupported for kind {}",
                    super::kind_label(&other)
                ),
            }),
        },
        PathSelector::Unrecognized => Err(ManipulationError::PathSegmentInvalid {
            message: format!("unrecognized segment {label}"),
        }),
    }
}

/// Applies a selector's selected values: terminal → all of them are results;
/// otherwise each continues the traversal and results are flattened.
fn selector_get<'a>(
    selected: Vec<&'a dyn Reflect>,
    segments: &RecursiveDescentSegments,
    cursor: &PathCursor,
) -> Result<Vec<&'a dyn Reflect>, ManipulationError> {
    if cursor.is_terminal(segments) {
        return Ok(selected);
    }
    let mut results = Vec::new();
    for value in selected {
        // a child that cannot continue the path simply yields no results
        // (go: an invalid recursiveGet result is skipped in the flatten loop)
        let child = if cursor.is_group_end(segments) {
            descent_get(value, segments, &cursor.next_group())
        } else {
            linear_get(value, segments, &cursor.next_in_group())
        };
        if let Ok(child_results) = child {
            results.extend(child_results);
        }
    }
    Ok(results)
}

/// Selects union members: keys against maps/structs, indices against sequences.
fn union_select<'a>(
    value: &'a dyn Reflect,
    members: &[dyn_rt_path::PathSegment],
) -> Result<Vec<&'a dyn Reflect>, ManipulationError> {
    let mut selected = Vec::new();
    match value.reflect_ref() {
        ReflectRef::Map(map) => {
            for member in members {
                match &member.selector {
                    PathSelector::Key(key) => {
                        let key_ref = convert_path_key(key, map.reflect_key_type_info())?;
                        if let Some(entry) = map.reflect_get(&*key_ref) {
                            selected.push(entry);
                        }
                    }
                    PathSelector::Wildcard => {
                        for (_, entry) in map.reflect_iter() {
                            selected.push(entry);
                        }
                    }
                    _ => {}
                }
            }
        }
        ReflectRef::Sequence(sequence) => {
            // index (incl. negative), slice and wildcard members all resolve
            // to concrete indices
            let indices = super::union_member_indices(members, sequence.reflect_length())?;
            for idx in indices {
                if let Some(element) = sequence.reflect_get(idx) {
                    selected.push(element);
                }
            }
        }
        ReflectRef::Struct(structure) => {
            for member in members {
                match &member.selector {
                    PathSelector::Key(_) | PathSelector::Index(_) => {
                        if let Some(field) =
                            super::struct_field_by_selector(structure, &member.selector)
                        {
                            selected.push(field);
                        }
                    }
                    PathSelector::Wildcard => {
                        let count = structure.reflect_field_count();
                        for index in 0..count {
                            if let Some(field) = structure.reflect_field_at(index) {
                                selected.push(field);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        other => {
            return Err(ManipulationError::PathSegmentInvalid {
                message: format!(
                    "union segment unsupported for kind {}",
                    super::kind_label(&other)
                ),
            });
        }
    }
    Ok(selected)
}

/// Recursive descent (`..`): searches the current group's segment at the
/// current level and all deeper levels.
fn descent_get<'a>(
    value: &'a dyn Reflect,
    segments: &RecursiveDescentSegments,
    cursor: &PathCursor,
) -> Result<Vec<&'a dyn Reflect>, ManipulationError> {
    let segment = &segments.0[cursor.recursive].0[cursor.collection];
    let label = selector_label(&segment.selector);

    if let Some(inner) = unwrap(value) {
        return descent_get(inner, segments, cursor);
    }

    let key = match &segment.selector {
        PathSelector::Root => return linear_get(value, segments, cursor),
        PathSelector::Key(key) => key.clone(),
        _ => {
            return Err(ManipulationError::PathSegmentInvalid {
                message: format!("recursive descent search segment {label} is not a key"),
            });
        }
    };

    let mut results = Vec::new();
    match value.reflect_ref() {
        ReflectRef::Map(map) => {
            for (map_key, entry) in map.reflect_iter() {
                if key_matches(map_key, &key) {
                    if let Ok(matched) = advance_get(entry, segments, cursor) {
                        results.extend(matched);
                    }
                }
                // leaves (scalars) simply yield nothing at this level
                if let Ok(deeper) = descent_get(entry, segments, cursor) {
                    results.extend(deeper);
                }
            }
        }
        ReflectRef::Sequence(sequence) => {
            for element in sequence.reflect_iter() {
                if let Ok(deeper) = descent_get(element, segments, cursor) {
                    results.extend(deeper);
                }
            }
        }
        ReflectRef::Struct(structure) => {
            let infos = structure.reflect_field_infos();
            for (index, info) in infos.iter().enumerate() {
                if info.name == key {
                    if let Some(field) = structure.reflect_field_at(index) {
                        if let Ok(matched) = advance_get(field, segments, cursor) {
                            results.extend(matched);
                        }
                    }
                }
                if let Some(field) = structure.reflect_field_at(index) {
                    if let Ok(deeper) = descent_get(field, segments, cursor) {
                        results.extend(deeper);
                    }
                }
            }
        }
        // scalar/unknown leaf: nothing to search at this level
        _ => {}
    }
    Ok(results)
}
