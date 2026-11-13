//! `delete`: remove the value(s) at a JSONPath from a reflected source.
//!
//! Port of go-json's `object.Delete` / `recursiveDelete` /
//! `recursiveDescentDelete`. Deletion is parent-side: a map key is removed
//! with `reflect_delete_key`, a sequence element with `reflect_delete_index`
//! (descending, so indices stay valid), a struct field is reset to its zero
//! value. Returns the number of values deleted.

use dyn_rt_json_path::{PathSelector, RecursiveDescentSegments};
use dyn_rt_reflect::{Reflect, ReflectRefMut};

use super::{
    convert_path_key, is_root_only, key_matches, resolve_index, selector_label, slice_indices,
};
use crate::domain::PathCursor;
use crate::error::ManipulationError;

/// Deletes the value(s) in `source` at `segments`, returning how many were
/// deleted. A lone `$`/empty path resets the source to its zero value.
pub fn delete(
    source: &mut dyn Reflect,
    segments: &RecursiveDescentSegments,
) -> Result<usize, ManipulationError> {
    if is_root_only(segments) {
        let zero =
            source
                .reflect_zero()
                .ok_or_else(|| ManipulationError::ValueAtPathSegmentInvalid {
                    message: format!(
                        "cannot reset root to zero: no zero constructor for {}",
                        source.reflect_type_name()
                    ),
                })?;
        source.reflect_apply(&*zero).map_err(|err| {
            ManipulationError::ValueAtPathSegmentInvalid {
                message: format!("reset root failed: {err}"),
            }
        })?;
        return Ok(1);
    }

    let mut count = 0usize;
    let cursor = PathCursor::start(segments.0.len());
    if cursor.recursive == cursor.last_recursive {
        linear_delete(source, segments, &cursor, &mut count)?;
    } else {
        descent_delete(source, segments, &cursor, &mut count)?;
    }
    Ok(count)
}

/// Linear traversal (no `..`); deletes at the current segment.
fn linear_delete(
    value: &mut dyn Reflect,
    segments: &RecursiveDescentSegments,
    cursor: &PathCursor,
    count: &mut usize,
) -> Result<(), ManipulationError> {
    let segment = &segments.0[cursor.recursive].0[cursor.collection];
    let label = selector_label(&segment.selector);
    let terminal = cursor.is_terminal(segments);

    // unwrap indirections / Some payloads; an empty wrapper (e.g. None) is an error here
    if let Some(inner) = super::unwrap_mut(value) {
        return linear_delete(inner, segments, cursor, count);
    }
    if matches!(
        value.reflect_ref(),
        dyn_rt_reflect::ReflectRef::Indirection(_) | dyn_rt_reflect::ReflectRef::Enum(_)
    ) {
        return Err(ManipulationError::ValueAtPathSegmentInvalid {
            message: format!("value nil or invalid at segment {label}"),
        });
    }

    match value.reflect_ref_mut() {
        ReflectRefMut::Map(map) => match &segment.selector {
            PathSelector::Root => advance_delete(value, segments, cursor, count),
            PathSelector::Key(key) => {
                let key_ref = convert_path_key(key, map.reflect_key_type_info())?;
                if !map.reflect_contains_key(&*key_ref) {
                    // deleting a non-existent key: nothing to do
                    return Ok(());
                }
                if terminal {
                    map.reflect_delete_key(&*key_ref).map_err(|err| {
                        ManipulationError::ValueAtPathSegmentInvalid {
                            message: format!("delete map entry {key:?} failed: {err}"),
                        }
                    })?;
                    *count += 1;
                    Ok(())
                } else {
                    let entry = map.reflect_get_mut(&*key_ref).ok_or_else(|| {
                        ManipulationError::ValueAtPathSegmentInvalid {
                            message: format!("map entry {key:?} vanished"),
                        }
                    })?;
                    linear_delete(entry, segments, &cursor.next_in_group(), count)
                }
            }
            PathSelector::Wildcard => {
                let keys: Vec<Box<dyn Reflect>> =
                    map.reflect_iter().map(|(k, _)| k.reflect_clone()).collect();
                for key in keys {
                    if terminal {
                        map.reflect_delete_key(&*key).map_err(|err| {
                            ManipulationError::ValueAtPathSegmentInvalid {
                                message: format!("delete map entry failed: {err}"),
                            }
                        })?;
                        *count += 1;
                    } else if let Some(entry) = map.reflect_get_mut(&*key) {
                        linear_delete(&mut *entry, segments, &cursor.next_in_group(), count)?;
                    }
                }
                Ok(())
            }
            PathSelector::Union(members) => {
                for member in members {
                    if let PathSelector::Key(key) = &member.selector {
                        let key_ref = convert_path_key(key, map.reflect_key_type_info())?;
                        if !map.reflect_contains_key(&*key_ref) {
                            continue;
                        }
                        if terminal {
                            map.reflect_delete_key(&*key_ref).map_err(|err| {
                                ManipulationError::ValueAtPathSegmentInvalid {
                                    message: format!("delete map entry {key:?} failed: {err}"),
                                }
                            })?;
                            *count += 1;
                        } else if let Some(entry) = map.reflect_get_mut(&*key_ref) {
                            linear_delete(&mut *entry, segments, &cursor.next_in_group(), count)?;
                        }
                    }
                }
                Ok(())
            }
            _ => Err(ManipulationError::PathSegmentInvalid {
                message: format!("segment {label} unsupported for kind map"),
            }),
        },
        ReflectRefMut::Sequence(sequence) => match &segment.selector {
            PathSelector::Root => advance_delete(value, segments, cursor, count),
            PathSelector::Index(index) => {
                let idx = resolve_index(*index, sequence.reflect_length())?;
                if terminal {
                    sequence.reflect_delete_index(idx).map_err(|err| {
                        ManipulationError::ValueAtPathSegmentInvalid {
                            message: format!("delete index {idx} failed: {err}"),
                        }
                    })?;
                    *count += 1;
                    Ok(())
                } else {
                    let mut owned = sequence
                        .reflect_get(idx)
                        .ok_or_else(|| ManipulationError::ValueAtPathSegmentInvalid {
                            message: format!("index {idx} out of range"),
                        })?
                        .reflect_clone();
                    linear_delete(&mut *owned, segments, &cursor.next_in_group(), count)?;
                    sequence.reflect_set_index(idx, owned).map_err(|err| {
                        ManipulationError::ValueAtPathSegmentInvalid {
                            message: format!("write back index {idx} failed: {err}"),
                        }
                    })?;
                    Ok(())
                }
            }
            PathSelector::Wildcard => {
                let len = sequence.reflect_length();
                let indices: Vec<usize> = (0..len).collect();
                selector_delete_indices(sequence, indices, segments, cursor, terminal, count)
            }
            PathSelector::Union(members) => {
                let len = sequence.reflect_length();
                let mut indices = Vec::new();
                for member in members {
                    if let PathSelector::Index(index) = &member.selector {
                        if let Ok(idx) = resolve_index(*index, len) {
                            indices.push(idx);
                        }
                    }
                }
                indices.sort_unstable_by(|a, b| b.cmp(a));
                indices.dedup();
                selector_delete_indices(sequence, indices, segments, cursor, terminal, count)
            }
            PathSelector::Slice(slice) => {
                let mut indices = slice_indices(slice, sequence.reflect_length())?;
                indices.sort_unstable_by(|a, b| b.cmp(a));
                selector_delete_indices(sequence, indices, segments, cursor, terminal, count)
            }
            _ => Err(ManipulationError::PathSegmentInvalid {
                message: format!("segment {label} unsupported for kind sequence"),
            }),
        },
        ReflectRefMut::Struct(structure) => match &segment.selector {
            PathSelector::Root => advance_delete(value, segments, cursor, count),
            PathSelector::Key(key) => {
                let field_type = structure.reflect_field_type_info(key);
                let Some(field) = structure.reflect_field_mut(key) else {
                    return Ok(());
                };
                if terminal {
                    zero_field(field, field_type);
                    *count += 1;
                    Ok(())
                } else {
                    linear_delete(field, segments, &cursor.next_in_group(), count)
                }
            }
            PathSelector::Wildcard => {
                let infos = structure.reflect_field_infos();
                for (index, info) in infos.iter().enumerate() {
                    let field_type = info.type_info;
                    let Some(field) = structure.reflect_field_at_mut(index) else {
                        continue;
                    };
                    if terminal {
                        zero_field(field, Some(field_type));
                        *count += 1;
                    } else {
                        linear_delete(field, segments, &cursor.next_in_group(), count)?;
                    }
                }
                Ok(())
            }
            PathSelector::Union(members) => {
                for member in members {
                    if let PathSelector::Key(key) = &member.selector {
                        let field_type = structure.reflect_field_type_info(key);
                        let Some(field) = structure.reflect_field_mut(key) else {
                            continue;
                        };
                        if terminal {
                            zero_field(field, field_type);
                            *count += 1;
                        } else {
                            linear_delete(field, segments, &cursor.next_in_group(), count)?;
                        }
                    }
                }
                Ok(())
            }
            _ => Err(ManipulationError::PathSegmentInvalid {
                message: format!("segment {label} unsupported for kind struct"),
            }),
        },
        other => Err(ManipulationError::PathSegmentInvalid {
            message: format!(
                "segment {label} unsupported for kind {}",
                super::kind_label_mut(&other)
            ),
        }),
    }
}

/// Deletes or recurses into each selected sequence index (indices are
/// pre-sorted descending so deletions don't shift later indices).
fn selector_delete_indices(
    sequence: &mut dyn dyn_rt_reflect::ReflectSequence,
    indices: Vec<usize>,
    segments: &RecursiveDescentSegments,
    cursor: &PathCursor,
    terminal: bool,
    count: &mut usize,
) -> Result<(), ManipulationError> {
    for idx in indices {
        if terminal {
            sequence.reflect_delete_index(idx).map_err(|err| {
                ManipulationError::ValueAtPathSegmentInvalid {
                    message: format!("delete index {idx} failed: {err}"),
                }
            })?;
            *count += 1;
        } else {
            let Some(element) = sequence.reflect_get(idx) else {
                continue;
            };
            let mut owned = element.reflect_clone();
            linear_delete(&mut *owned, segments, &cursor.next_in_group(), count)?;
            sequence.reflect_set_index(idx, owned).map_err(|err| {
                ManipulationError::ValueAtPathSegmentInvalid {
                    message: format!("write back index {idx} failed: {err}"),
                }
            })?;
        }
    }
    Ok(())
}

/// Advances past the current segment (used for the root segment).
fn advance_delete(
    value: &mut dyn Reflect,
    segments: &RecursiveDescentSegments,
    cursor: &PathCursor,
    count: &mut usize,
) -> Result<(), ManipulationError> {
    if cursor.is_terminal(segments) {
        let zero =
            value
                .reflect_zero()
                .ok_or_else(|| ManipulationError::ValueAtPathSegmentInvalid {
                    message: format!(
                        "cannot reset to zero: no zero constructor for {}",
                        value.reflect_type_name()
                    ),
                })?;
        value.reflect_apply(&*zero).map_err(|err| {
            ManipulationError::ValueAtPathSegmentInvalid {
                message: format!("reset failed: {err}"),
            }
        })?;
        *count += 1;
        Ok(())
    } else if cursor.is_group_end(segments) {
        descent_delete(value, segments, &cursor.next_group(), count)
    } else {
        linear_delete(value, segments, &cursor.next_in_group(), count)
    }
}

/// Resets a struct field to its zero value (via the field type's constructor
/// or the value's own zero constructor).
fn zero_field(field: &mut dyn Reflect, type_info: Option<&'static dyn_rt_reflect::TypeInfo>) {
    let zero = type_info
        .and_then(|info| info.construct())
        .or_else(|| field.reflect_zero());
    if let Some(zero) = zero {
        let _ = field.reflect_apply(&*zero);
    }
}

/// Recursive descent (`..`) delete.
fn descent_delete(
    value: &mut dyn Reflect,
    segments: &RecursiveDescentSegments,
    cursor: &PathCursor,
    count: &mut usize,
) -> Result<(), ManipulationError> {
    let segment = &segments.0[cursor.recursive].0[cursor.collection];
    let label = selector_label(&segment.selector);

    if let Some(inner) = super::unwrap_mut(value) {
        return descent_delete(inner, segments, cursor, count);
    }
    if matches!(
        value.reflect_ref(),
        dyn_rt_reflect::ReflectRef::Indirection(_) | dyn_rt_reflect::ReflectRef::Enum(_)
    ) {
        return Err(ManipulationError::ValueAtPathSegmentInvalid {
            message: format!("value nil or invalid at recursive descent {label}"),
        });
    }

    let key = match &segment.selector {
        PathSelector::Root => return linear_delete(value, segments, cursor, count),
        PathSelector::Key(key) => key.clone(),
        _ => {
            return Err(ManipulationError::PathSegmentInvalid {
                message: format!("recursive descent search segment {label} is not a key"),
            });
        }
    };

    match value.reflect_ref_mut() {
        ReflectRefMut::Map(map) => {
            let keys: Vec<Box<dyn Reflect>> =
                map.reflect_iter().map(|(k, _)| k.reflect_clone()).collect();
            for map_key in keys {
                let matches = key_matches(&*map_key, &key);
                if matches && cursor.is_terminal(segments) {
                    // terminal match: delete the entry from the parent map
                    map.reflect_delete_key(&*map_key).map_err(|err| {
                        ManipulationError::ValueAtPathSegmentInvalid {
                            message: format!("delete map entry {key:?} failed: {err}"),
                        }
                    })?;
                    *count += 1;
                    continue;
                }
                let Some(entry) = map.reflect_get_mut(&*map_key) else {
                    continue;
                };
                if matches {
                    if cursor.is_group_end(segments) {
                        descent_delete(&mut *entry, segments, &cursor.next_group(), count)?;
                    } else {
                        linear_delete(&mut *entry, segments, &cursor.next_in_group(), count)?;
                    }
                }
                // keep descending into the entry at the same level
                descent_delete(&mut *entry, segments, cursor, count)?;
            }
            Ok(())
        }
        ReflectRefMut::Sequence(sequence) => {
            let len = sequence.reflect_length();
            for idx in (0..len).rev() {
                let Some(element) = sequence.reflect_get(idx) else {
                    continue;
                };
                let mut owned = element.reflect_clone();
                descent_delete(&mut *owned, segments, cursor, count)?;
                sequence.reflect_set_index(idx, owned).map_err(|err| {
                    ManipulationError::ValueAtPathSegmentInvalid {
                        message: format!("write back index {idx} failed: {err}"),
                    }
                })?;
            }
            Ok(())
        }
        ReflectRefMut::Struct(structure) => {
            let field_count = structure.reflect_field_count();
            for index in 0..field_count {
                let info = structure.reflect_field_infos()[index];
                let Some(field) = structure.reflect_field_at_mut(index) else {
                    continue;
                };
                if info.name == key && cursor.is_terminal(segments) {
                    zero_field(field, Some(info.type_info));
                    *count += 1;
                    continue;
                }
                if info.name == key {
                    if cursor.is_group_end(segments) {
                        descent_delete(field, segments, &cursor.next_group(), count)?;
                    } else {
                        linear_delete(field, segments, &cursor.next_in_group(), count)?;
                    }
                }
                descent_delete(field, segments, cursor, count)?;
            }
            Ok(())
        }
        other => Err(ManipulationError::ValueAtPathSegmentInvalid {
            message: format!(
                "unsupported value at recursive descent search segment {label} (kind {})",
                super::kind_label_mut(&other)
            ),
        }),
    }
}
