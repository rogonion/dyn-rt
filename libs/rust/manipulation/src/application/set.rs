//! `set`: create or update the value(s) at a JSONPath in a reflected source.
//!
//! Port of go-json's `object.Set` / `recursiveSet` / `getDefaultValueAtPathSegment`
//! / `recursiveDescentSet`. The value being set is coerced into the destination
//! type via schema conversion; when a schema is provided, missing
//! intermediate structures (maps, structs, sequence elements, indirection
//! inners) are created from the schema node at the current path.
//!
//! Returns the number of values set.

use dyn_rt_path::{PathSegment, PathSelector, RecursiveDescentSegment, RecursiveDescentSegments};
use dyn_rt_reflect::{Reflect, ReflectRef, ReflectRefMut};
use dyn_rt_schema::{Schema, SchemaNode, convert_node, schema_at_path};

use super::{
    convert_path_key, is_root_only, key_matches, resolve_set_index, selector_label, slice_indices,
};
use crate::domain::PathCursor;
use crate::error::ManipulationError;

/// Sets `value` at `segments` in `source`, optionally guided by `schema` for
/// type coercion and intermediate-structure creation. A lone `$`/empty path
/// replaces the source (via `reflect_apply`). Returns how many values were set.
pub fn set(
    source: &mut dyn Reflect,
    segments: &RecursiveDescentSegments,
    value: &dyn Reflect,
    schema: Option<&Schema>,
) -> Result<usize, ManipulationError> {
    if is_root_only(segments) {
        source
            .reflect_apply(value)
            .map_err(|err| ManipulationError::ConversionFailed {
                message: format!("set root failed: {err}"),
            })?;
        return Ok(1);
    }

    let mut count = 0usize;
    let mut path: Vec<PathSegment> = Vec::new();
    let cursor = PathCursor::start(segments.0.len());
    if cursor.recursive == cursor.last_recursive {
        linear_set(
            source, segments, &cursor, value, schema, &mut path, &mut count,
        )?;
    } else {
        descent_set(
            source, segments, &cursor, value, schema, &mut path, &mut count,
        )?;
    }
    Ok(count)
}

/// The schema node at `path`, when a schema was provided and resolves.
fn schema_node_at(path: &[PathSegment], schema: Option<&Schema>) -> Option<SchemaNode> {
    let schema = schema?;
    schema_at_path(&RecursiveDescentSegment(path.to_vec()), schema)
        .ok()
        .map(|resolved| resolved.node)
}

/// A fresh default value for a schema node: its `default` fn when present,
/// else the node type's `ReflectNew` constructor.
fn default_for_node(node: Option<&SchemaNode>) -> Option<Box<dyn Reflect>> {
    match node {
        Some(node) => {
            if let Some(default) = node.default {
                return Some(default());
            }
            node.type_info.construct()
        }
        None => None,
    }
}

/// A fallback schema node for a value of the given type: scalar nodes for
/// scalar types, `Any` (pass-through) otherwise.
fn fallback_node(type_info: &'static dyn_rt_reflect::TypeInfo) -> SchemaNode {
    match type_info.kind {
        dyn_rt_reflect::ReflectKind::Scalar => {
            super::scalar_node_for(type_info).unwrap_or_else(SchemaNode::any)
        }
        _ => SchemaNode::any(),
    }
}

/// Converts `value` into `target` in place, using the schema node at `path`
/// when available and a fallback node for the target's type otherwise.
fn convert_into(
    value: &dyn Reflect,
    target: &mut dyn Reflect,
    path: &[PathSegment],
    schema: Option<&Schema>,
    target_type_info: &'static dyn_rt_reflect::TypeInfo,
) -> Result<(), ManipulationError> {
    let node = schema_node_at(path, schema).unwrap_or_else(|| fallback_node(target_type_info));
    convert_node(value, &node, target).map_err(|err| ManipulationError::ConversionFailed {
        message: format!(
            "convert value into {} failed: {err}",
            target_type_info.type_name
        ),
    })
}

/// Builds a converted, owned copy of `value` for the given destination node.
fn convert_to_owned(
    value: &dyn Reflect,
    node: Option<&SchemaNode>,
    path: &[PathSegment],
    schema: Option<&Schema>,
    dest_type_info: &'static dyn_rt_reflect::TypeInfo,
) -> Result<Box<dyn Reflect>, ManipulationError> {
    let node = match node {
        Some(node) => node.clone(),
        None => schema_node_at(path, schema).unwrap_or_else(|| fallback_node(dest_type_info)),
    };
    let mut instance =
        default_for_node(Some(&node)).ok_or_else(|| ManipulationError::ConversionFailed {
            message: format!(
                "no constructor for destination type {}",
                dest_type_info.type_name
            ),
        })?;
    convert_node(value, &node, &mut *instance).map_err(|err| {
        ManipulationError::ConversionFailed {
            message: format!(
                "convert value into {} failed: {err}",
                dest_type_info.type_name
            ),
        }
    })?;
    Ok(instance)
}

/// Ensures a map entry exists, creating it with a default value when absent.
fn ensure_map_entry(
    map: &mut dyn dyn_rt_reflect::ReflectMap,
    key_ref: &dyn Reflect,
    path: &[PathSegment],
    segment: &PathSegment,
    schema: Option<&Schema>,
) -> Result<(), ManipulationError> {
    if map.reflect_contains_key(key_ref) {
        return Ok(());
    }
    let mut entry_path = path.to_vec();
    entry_path.push(segment.clone());
    let node = schema_node_at(&entry_path, schema);
    let default = default_for_node(node.as_ref())
        .or_else(|| super::construct_value(map.reflect_value_type_info()))
        .or_else(|| map.reflect_zero())
        .ok_or_else(|| ManipulationError::ValueAtPathSegmentInvalid {
            message: "cannot create missing map entry: no default for the value type".to_string(),
        })?;
    map.reflect_set_key(key_ref, default).map_err(|err| {
        ManipulationError::ValueAtPathSegmentInvalid {
            message: format!("create map entry failed: {err}"),
        }
    })
}

/// Linear traversal (no `..`); sets at the current segment.
fn linear_set(
    value: &mut dyn Reflect,
    segments: &RecursiveDescentSegments,
    cursor: &PathCursor,
    value_to_set: &dyn Reflect,
    schema: Option<&Schema>,
    path: &mut Vec<PathSegment>,
    count: &mut usize,
) -> Result<(), ManipulationError> {
    let segment = segments.0[cursor.recursive].0[cursor.collection].clone();
    let label = selector_label(&segment.selector);
    let terminal = cursor.is_terminal(segments);

    // unwrap indirections / Some payloads; create the inner value of an empty
    // indirection when the schema (or a constructor) tells us what it should be
    if let Some(inner) = super::unwrap_mut(value) {
        return linear_set(inner, segments, cursor, value_to_set, schema, path, count);
    }
    if matches!(value.reflect_ref(), ReflectRef::Indirection(_)) {
        let mut entry_path = path.clone();
        entry_path.push(segment.clone());
        // empty indirection: create the inner from the schema/default
        let node = schema_node_at(&entry_path, schema);
        let inner_default = default_for_node(node.as_ref())
            .or_else(|| {
                node.as_ref()
                    .and_then(|n| super::construct_value(&n.type_info))
            })
            .ok_or_else(|| ManipulationError::ValueAtPathSegmentInvalid {
                message: format!("cannot create inner value at segment {label}"),
            })?;
        if let ReflectRefMut::Indirection(indirection) = value.reflect_ref_mut() {
            indirection
                .reflect_set_inner(inner_default)
                .map_err(|err| ManipulationError::ValueAtPathSegmentInvalid {
                    message: format!("set inner failed: {err}"),
                })?;
        }
        let inner = super::unwrap_mut(value).ok_or_else(|| {
            ManipulationError::ValueAtPathSegmentInvalid {
                message: format!("inner still empty after creation at segment {label}"),
            }
        })?;
        return linear_set(inner, segments, cursor, value_to_set, schema, path, count);
    }
    if matches!(value.reflect_ref(), ReflectRef::Enum(_)) {
        return Err(ManipulationError::ValueAtPathSegmentInvalid {
            message: format!("value nil or invalid at segment {label}"),
        });
    }

    match value.reflect_ref_mut() {
        ReflectRefMut::Map(map) => {
            let value_type = map.reflect_value_type_info();
            match &segment.selector {
                PathSelector::Root => {
                    advance_set(value, segments, cursor, value_to_set, schema, path, count)
                }
                PathSelector::Key(key) => {
                    let key_ref = convert_path_key(key, map.reflect_key_type_info())?;
                    ensure_map_entry(map, &*key_ref, path, &segment, schema)?;
                    let entry = map.reflect_get_mut(&*key_ref).ok_or_else(|| {
                        ManipulationError::ValueAtPathSegmentInvalid {
                            message: format!("map entry {key:?} vanished"),
                        }
                    })?;
                    let mut entry_path = path.clone();
                    entry_path.push(segment.clone());
                    if terminal {
                        let entry_type = value_type;
                        let result =
                            convert_into(value_to_set, entry, &entry_path, schema, entry_type);
                        if result.is_ok() {
                            *count += 1;
                        }
                        result
                    } else {
                        path.push(segment.clone());
                        let result = linear_set(
                            entry,
                            segments,
                            &cursor.next_in_group(),
                            value_to_set,
                            schema,
                            path,
                            count,
                        );
                        path.pop();
                        result
                    }
                }
                PathSelector::Wildcard => {
                    let keys: Vec<Box<dyn Reflect>> =
                        map.reflect_iter().map(|(k, _)| k.reflect_clone()).collect();
                    for key in keys {
                        ensure_map_entry(map, &*key, path, &segment, schema)?;
                        let Some(entry) = map.reflect_get_mut(&*key) else {
                            continue;
                        };
                        let mut entry_path = path.clone();
                        entry_path.push(segment.clone());
                        if terminal {
                            let entry_type = value_type;
                            if convert_into(value_to_set, entry, &entry_path, schema, entry_type)
                                .is_ok()
                            {
                                *count += 1;
                            }
                        } else {
                            path.push(segment.clone());
                            linear_set(
                                entry,
                                segments,
                                &cursor.next_in_group(),
                                value_to_set,
                                schema,
                                path,
                                count,
                            )?;
                            path.pop();
                        }
                    }
                    Ok(())
                }
                PathSelector::Union(members) => {
                    for member in members {
                        if let PathSelector::Key(key) = &member.selector {
                            let member_segment = PathSegment {
                                access_strategy: member.access_strategy,
                                selector: PathSelector::Key(key.clone()),
                            };
                            let key_ref = convert_path_key(key, map.reflect_key_type_info())?;
                            ensure_map_entry(map, &*key_ref, path, &member_segment, schema)?;
                            let Some(entry) = map.reflect_get_mut(&*key_ref) else {
                                continue;
                            };
                            let mut entry_path = path.clone();
                            entry_path.push(member_segment.clone());
                            if terminal {
                                let entry_type = value_type;
                                if convert_into(
                                    value_to_set,
                                    entry,
                                    &entry_path,
                                    schema,
                                    entry_type,
                                )
                                .is_ok()
                                {
                                    *count += 1;
                                }
                            } else {
                                path.push(member_segment.clone());
                                linear_set(
                                    entry,
                                    segments,
                                    &cursor.next_in_group(),
                                    value_to_set,
                                    schema,
                                    path,
                                    count,
                                )?;
                                path.pop();
                            }
                        }
                    }
                    Ok(())
                }
                _ => Err(ManipulationError::PathSegmentInvalid {
                    message: format!("segment {label} unsupported for kind map"),
                }),
            }
        }
        ReflectRefMut::Sequence(sequence) => match &segment.selector {
            PathSelector::Root => {
                advance_set(value, segments, cursor, value_to_set, schema, path, count)
            }
            PathSelector::Index(index) => {
                // unlike get, a set index may be past the end (expansion)
                let idx = resolve_set_index(*index, sequence.reflect_length())?;
                let mut walk = SetWalk {
                    segments,
                    cursor,
                    value_to_set,
                    schema,
                    path,
                    segment: &segment,
                    terminal,
                    count,
                };
                set_sequence_index(sequence, idx, &mut walk)
            }
            PathSelector::Wildcard => {
                let len = sequence.reflect_length();
                let indices: Vec<usize> = (0..len).collect();
                let mut walk = SetWalk {
                    segments,
                    cursor,
                    value_to_set,
                    schema,
                    path,
                    segment: &segment,
                    terminal,
                    count,
                };
                set_sequence_indices(sequence, indices, &mut walk)
            }
            PathSelector::Union(members) => {
                // index (incl. negative), slice and wildcard members
                let indices = super::union_member_indices(members, sequence.reflect_length())?;
                let mut walk = SetWalk {
                    segments,
                    cursor,
                    value_to_set,
                    schema,
                    path,
                    segment: &segment,
                    terminal,
                    count,
                };
                set_sequence_indices(sequence, indices, &mut walk)
            }
            PathSelector::Slice(slice) => {
                let indices = slice_indices(slice, sequence.reflect_length())?;
                let mut walk = SetWalk {
                    segments,
                    cursor,
                    value_to_set,
                    schema,
                    path,
                    segment: &segment,
                    terminal,
                    count,
                };
                set_sequence_indices(sequence, indices, &mut walk)
            }
            _ => Err(ManipulationError::PathSegmentInvalid {
                message: format!("segment {label} unsupported for kind sequence"),
            }),
        },
        ReflectRefMut::Struct(structure) => match &segment.selector {
            PathSelector::Root => {
                advance_set(value, segments, cursor, value_to_set, schema, path, count)
            }
            // keys by name; non-negative indices by positional field name
            // (tuples are struct-shaped, so `$[1]` sets tuple element "1")
            PathSelector::Key(_) | PathSelector::Index(_) => {
                let field_type =
                    super::struct_field_type_info_by_selector(&*structure, &segment.selector)
                        .ok_or_else(|| ManipulationError::ValueAtPathSegmentInvalid {
                            message: format!("no type info for struct field {label}"),
                        })?;
                let Some(field) = super::struct_field_by_selector_mut(structure, &segment.selector)
                else {
                    return Err(ManipulationError::ValueAtPathSegmentInvalid {
                        message: format!("struct field {label} not found"),
                    });
                };
                let mut field_path = path.clone();
                field_path.push(segment.clone());
                if terminal {
                    let result = convert_into(value_to_set, field, &field_path, schema, field_type);
                    if result.is_ok() {
                        *count += 1;
                    }
                    result
                } else {
                    path.push(segment.clone());
                    let result = linear_set(
                        field,
                        segments,
                        &cursor.next_in_group(),
                        value_to_set,
                        schema,
                        path,
                        count,
                    );
                    path.pop();
                    result
                }
            }
            PathSelector::Wildcard => {
                let infos = structure.reflect_field_infos();
                for (index, info) in infos.iter().enumerate() {
                    let Some(field) = structure.reflect_field_at_mut(index) else {
                        continue;
                    };
                    let mut field_path = path.clone();
                    field_path.push(segment.clone());
                    if terminal {
                        if convert_into(value_to_set, field, &field_path, schema, info.type_info)
                            .is_ok()
                        {
                            *count += 1;
                        }
                    } else {
                        path.push(segment.clone());
                        linear_set(
                            field,
                            segments,
                            &cursor.next_in_group(),
                            value_to_set,
                            schema,
                            path,
                            count,
                        )?;
                        path.pop();
                    }
                }
                Ok(())
            }
            PathSelector::Union(members) => {
                for member in members {
                    // keys by name, indices by positional field name (tuples)
                    if matches!(
                        &member.selector,
                        PathSelector::Key(_) | PathSelector::Index(_)
                    ) {
                        let field_type = super::struct_field_type_info_by_selector(
                            &*structure,
                            &member.selector,
                        );
                        let Some(field) =
                            super::struct_field_by_selector_mut(structure, &member.selector)
                        else {
                            continue;
                        };
                        let mut field_path = path.clone();
                        field_path.push(segment.clone());
                        if terminal {
                            if let Some(info) = field_type {
                                if convert_into(value_to_set, field, &field_path, schema, info)
                                    .is_ok()
                                {
                                    *count += 1;
                                }
                            }
                        } else {
                            path.push(segment.clone());
                            linear_set(
                                field,
                                segments,
                                &cursor.next_in_group(),
                                value_to_set,
                                schema,
                                path,
                                count,
                            )?;
                            path.pop();
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

/// Walk state shared by the sequence setters — bundled so the helpers stay
/// under the arity lint while keeping the go-style explicit parameter flow.
struct SetWalk<'a> {
    segments: &'a RecursiveDescentSegments,
    cursor: &'a PathCursor,
    value_to_set: &'a dyn Reflect,
    schema: Option<&'a Schema>,
    path: &'a mut Vec<PathSegment>,
    segment: &'a PathSegment,
    terminal: bool,
    count: &'a mut usize,
}

/// Sets one sequence index: replaces in place, pushes at the end, or expands
/// with defaults up to the index (go's "set slice index with expansion").
fn set_sequence_index(
    sequence: &mut dyn dyn_rt_reflect::ReflectSequence,
    idx: usize,
    walk: &mut SetWalk,
) -> Result<(), ManipulationError> {
    let element_type = sequence.reflect_element_type_info();
    let mut index_path = walk.path.clone();
    index_path.push(walk.segment.clone());

    // expand with defaults when the index is past the end
    while sequence.reflect_length() <= idx {
        let node = schema_node_at(&index_path, walk.schema);
        let default = default_for_node(node.as_ref())
            .or_else(|| super::construct_value(element_type))
            .ok_or_else(|| ManipulationError::ValueAtPathSegmentInvalid {
                message: format!("cannot expand sequence to index {idx}: no default element"),
            })?;
        sequence.reflect_push(default).map_err(|err| {
            ManipulationError::ValueAtPathSegmentInvalid {
                message: format!("expand sequence failed: {err}"),
            }
        })?;
    }

    if walk.terminal {
        let converted = convert_to_owned(
            walk.value_to_set,
            schema_node_at(&index_path, walk.schema).as_ref(),
            &index_path,
            walk.schema,
            element_type,
        )?;
        sequence.reflect_set_index(idx, converted).map_err(|err| {
            ManipulationError::ValueAtPathSegmentInvalid {
                message: format!("set index {idx} failed: {err}"),
            }
        })?;
        *walk.count += 1;
        Ok(())
    } else {
        let Some(element) = sequence.reflect_get(idx) else {
            return Err(ManipulationError::ValueAtPathSegmentInvalid {
                message: format!("index {idx} out of range after expansion"),
            });
        };
        let mut owned = element.reflect_clone();
        walk.path.push(walk.segment.clone());
        let result = linear_set(
            &mut *owned,
            walk.segments,
            &walk.cursor.next_in_group(),
            walk.value_to_set,
            walk.schema,
            walk.path,
            walk.count,
        );
        walk.path.pop();
        sequence.reflect_set_index(idx, owned).map_err(|err| {
            ManipulationError::ValueAtPathSegmentInvalid {
                message: format!("write back index {idx} failed: {err}"),
            }
        })?;
        result
    }
}

/// Sets each selected sequence index.
fn set_sequence_indices(
    sequence: &mut dyn dyn_rt_reflect::ReflectSequence,
    indices: Vec<usize>,
    walk: &mut SetWalk,
) -> Result<(), ManipulationError> {
    for idx in indices {
        set_sequence_index(sequence, idx, walk)?;
    }
    Ok(())
}

/// Advances past the current (root) segment.
fn advance_set(
    value: &mut dyn Reflect,
    segments: &RecursiveDescentSegments,
    cursor: &PathCursor,
    value_to_set: &dyn Reflect,
    schema: Option<&Schema>,
    path: &mut Vec<PathSegment>,
    count: &mut usize,
) -> Result<(), ManipulationError> {
    if cursor.is_terminal(segments) {
        value
            .reflect_apply(value_to_set)
            .map_err(|err| ManipulationError::ConversionFailed {
                message: format!("set root failed: {err}"),
            })?;
        *count += 1;
        Ok(())
    } else if cursor.is_group_end(segments) {
        descent_set(
            value,
            segments,
            &cursor.next_group(),
            value_to_set,
            schema,
            path,
            count,
        )
    } else {
        linear_set(
            value,
            segments,
            &cursor.next_in_group(),
            value_to_set,
            schema,
            path,
            count,
        )
    }
}

/// Recursive descent (`..`) set: applies the rest of the path at every level
/// where the search key matches, and keeps descending into all children.
fn descent_set(
    value: &mut dyn Reflect,
    segments: &RecursiveDescentSegments,
    cursor: &PathCursor,
    value_to_set: &dyn Reflect,
    schema: Option<&Schema>,
    path: &mut Vec<PathSegment>,
    count: &mut usize,
) -> Result<(), ManipulationError> {
    let segment = segments.0[cursor.recursive].0[cursor.collection].clone();
    let label = selector_label(&segment.selector);

    if let Some(inner) = super::unwrap_mut(value) {
        return descent_set(inner, segments, cursor, value_to_set, schema, path, count);
    }
    if matches!(
        value.reflect_ref(),
        ReflectRef::Indirection(_) | ReflectRef::Enum(_)
    ) {
        return Err(ManipulationError::ValueAtPathSegmentInvalid {
            message: format!("value nil or invalid at recursive descent {label}"),
        });
    }

    let key = match &segment.selector {
        PathSelector::Root => {
            return linear_set(value, segments, cursor, value_to_set, schema, path, count);
        }
        PathSelector::Key(key) => key.clone(),
        _ => {
            return Err(ManipulationError::PathSegmentInvalid {
                message: format!("recursive descent search segment {label} is not a key"),
            });
        }
    };

    match value.reflect_ref_mut() {
        ReflectRefMut::Map(map) => {
            let value_type = map.reflect_value_type_info();
            let keys: Vec<Box<dyn Reflect>> =
                map.reflect_iter().map(|(k, _)| k.reflect_clone()).collect();
            for map_key in keys {
                let matches = key_matches(&*map_key, &key);
                if matches {
                    // ensure the entry exists before descending/setting into it
                    ensure_map_entry(map, &*map_key, path, &segment, schema)?;
                }
                let Some(entry) = map.reflect_get_mut(&*map_key) else {
                    continue;
                };
                if matches {
                    path.push(segment.clone());
                    if cursor.is_terminal(segments) {
                        let entry_type = value_type;
                        let entry_path = path.clone();
                        let result =
                            convert_into(value_to_set, entry, &entry_path, schema, entry_type);
                        if result.is_ok() {
                            *count += 1;
                        }
                        path.pop();
                        result?;
                    } else if cursor.is_group_end(segments) {
                        let result = descent_set(
                            entry,
                            segments,
                            &cursor.next_group(),
                            value_to_set,
                            schema,
                            path,
                            count,
                        );
                        path.pop();
                        result?;
                    } else {
                        let result = linear_set(
                            entry,
                            segments,
                            &cursor.next_in_group(),
                            value_to_set,
                            schema,
                            path,
                            count,
                        );
                        path.pop();
                        result?;
                    }
                }
                // keep descending into the entry at the same level
                if !matches || !cursor.is_terminal(segments) {
                    let Some(entry) = map.reflect_get_mut(&*map_key) else {
                        continue;
                    };
                    descent_set(entry, segments, cursor, value_to_set, schema, path, count)?;
                }
            }
            Ok(())
        }
        ReflectRefMut::Sequence(sequence) => {
            let len = sequence.reflect_length();
            for idx in 0..len {
                let Some(element) = sequence.reflect_get(idx) else {
                    continue;
                };
                let mut owned = element.reflect_clone();
                descent_set(
                    &mut *owned,
                    segments,
                    cursor,
                    value_to_set,
                    schema,
                    path,
                    count,
                )?;
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
                if info.name == key {
                    path.push(segment.clone());
                    if cursor.is_terminal(segments) {
                        let result =
                            convert_into(value_to_set, field, path, schema, info.type_info);
                        path.pop();
                        if result.is_ok() {
                            *count += 1;
                        }
                        result?;
                    } else if cursor.is_group_end(segments) {
                        let result = descent_set(
                            field,
                            segments,
                            &cursor.next_group(),
                            value_to_set,
                            schema,
                            path,
                            count,
                        );
                        path.pop();
                        result?;
                    } else {
                        let result = linear_set(
                            field,
                            segments,
                            &cursor.next_in_group(),
                            value_to_set,
                            schema,
                            path,
                            count,
                        );
                        path.pop();
                        result?;
                    }
                }
                descent_set(field, segments, cursor, value_to_set, schema, path, count)?;
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
