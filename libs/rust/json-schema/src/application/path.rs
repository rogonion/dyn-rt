//! Schema-path resolution: look up the schema node for a JSONPath within a
//! larger schema (port of go-json's path.go). Paths must be linear (no
//! recursive descent `..`); the string entry point lives in adapters.

use dyn_rt_json_path::{PathSegment, PathSelector, RecursiveDescentSegment};

use crate::domain::{Schema, SchemaNode, SchemaSet, SchemaShape, render_path};
use crate::error::SchemaError;

// ---------------------------------------------------------------------------
// schema path resolution (port of path.go)
// ---------------------------------------------------------------------------

/// The result of resolving a path against a schema: the node at that path
/// plus the set keys that matched along the way.
#[derive(Debug, Clone)]
pub struct SchemaAtPath {
    pub node: SchemaNode,
    pub valid_set_keys: Vec<String>,
}

/// Resolves a single recursive-descent segment (a linear path, no `..`)
/// against `schema`. For path strings, see the adapters module.
pub fn schema_at_path(
    segment: &RecursiveDescentSegment,
    schema: &Schema,
) -> Result<SchemaAtPath, SchemaError> {
    let mut valid_set_keys = Vec::new();
    let node = resolve_at(0, &segment.0, schema, &mut valid_set_keys)?;
    Ok(SchemaAtPath {
        node,
        valid_set_keys,
    })
}

fn resolve_at(
    index: usize,
    segments: &[PathSegment],
    schema: &Schema,
    valid_set_keys: &mut Vec<String>,
) -> Result<SchemaNode, SchemaError> {
    if segments.is_empty() || index >= segments.len() {
        return Err(SchemaError::SchemaPathError {
            message: "current path segment indexes exhausted".to_string(),
            path: render_path(&[]),
        });
    }
    match schema {
        Schema::Set(set) => resolve_set_at(index, segments, set, valid_set_keys),
        Schema::Node(node) => resolve_node_at(index, segments, node, valid_set_keys),
    }
}

fn resolve_set_at(
    index: usize,
    segments: &[PathSegment],
    set: &SchemaSet,
    valid_set_keys: &mut Vec<String>,
) -> Result<SchemaNode, SchemaError> {
    let path = render_path(&[]);
    if set.nodes.is_empty() {
        return Err(SchemaError::SchemaPathError {
            message: "no schema nodes found".to_string(),
            path,
        });
    }
    let mut last_err: Option<SchemaError> = None;

    let mut try_node = |key: &str, node: &SchemaNode| -> Option<SchemaNode> {
        match resolve_node_at(index, segments, node, valid_set_keys) {
            Ok(resolved) => {
                valid_set_keys.push(key.to_string());
                Some(resolved)
            }
            Err(err) => {
                last_err = Some(err);
                None
            }
        }
    };

    if let Some(default_key) = &set.default_key {
        if let Some((_, node)) = set.nodes.iter().find(|(key, _)| key == default_key) {
            if let Some(resolved) = try_node(default_key, node) {
                return Ok(resolved);
            }
        }
    }
    for (key, node) in &set.nodes {
        if Some(key) == set.default_key.as_ref() {
            continue;
        }
        if let Some(resolved) = try_node(key, node) {
            return Ok(resolved);
        }
    }
    Err(last_err.unwrap_or_else(|| SchemaError::SchemaPathError {
        message: "no schema nodes found".to_string(),
        path,
    }))
}

fn resolve_node_at(
    index: usize,
    segments: &[PathSegment],
    node: &SchemaNode,
    valid_set_keys: &mut Vec<String>,
) -> Result<SchemaNode, SchemaError> {
    let last = segments.len() - 1;
    let segment = &segments[index];
    let path = render_path(&[]);
    let fail = |message: String| SchemaError::SchemaPathError {
        message,
        path: path.clone(),
    };

    match &segment.selector {
        PathSelector::Root => {
            if index == last {
                Ok(node.clone())
            } else {
                resolve_node_at(index + 1, segments, node, valid_set_keys)
            }
        }
        PathSelector::Key(key) => {
            resolve_with_collection_key(index, segments, node, key.clone(), valid_set_keys)
        }
        PathSelector::Index(position) => {
            resolve_with_collection_key(index, segments, node, position.to_string(), valid_set_keys)
        }
        _ => Err(fail("current path segment is not key or index".to_string())),
    }
}

fn resolve_with_collection_key(
    index: usize,
    segments: &[PathSegment],
    node: &SchemaNode,
    collection_key: String,
    valid_set_keys: &mut Vec<String>,
) -> Result<SchemaNode, SchemaError> {
    let last = segments.len() - 1;
    let fail = |message: String| SchemaError::SchemaPathError {
        message,
        path: render_path(&[]),
    };

    match &node.shape {
        // A pointer does not consume a path segment: resolve into the inner
        // schema at the same index.
        SchemaShape::Indirection { inner } => resolve_at(index, segments, inner, valid_set_keys),
        SchemaShape::Map {
            entries,
            key,
            value,
        } => {
            if let Some((_, entry_schema)) = entries.iter().find(|(k, _)| *k == collection_key) {
                if index == last {
                    return schema_as_node(entry_schema, valid_set_keys);
                }
                return resolve_at(index + 1, segments, entry_schema, valid_set_keys);
            }
            let (Some(key_schema), Some(value_schema)) = (key, value) else {
                return Err(fail(
                    "schema for associative collection keys and/or values not found".to_string(),
                ));
            };
            if index == last {
                // Synthesize the entry node: the value node carrying the key
                // schema (go sets AssociativeCollectionEntryKeySchema).
                let mut entry_node = schema_as_node(value_schema, valid_set_keys)?;
                entry_node.entry_key_schema = Some(Box::new((**key_schema).clone()));
                return Ok(entry_node);
            }
            resolve_at(index + 1, segments, value_schema, valid_set_keys)
        }
        SchemaShape::Sequence { elements, items } => {
            if let Some((_, item_schema)) = items
                .iter()
                .find(|(item_index, _)| item_index.to_string() == collection_key)
            {
                if index == last {
                    return schema_as_node(item_schema, valid_set_keys);
                }
                return resolve_at(index + 1, segments, item_schema, valid_set_keys);
            }
            let Some(elements) = elements else {
                return Err(fail(
                    "schema for linear collection elements not found".to_string(),
                ));
            };
            if index == last {
                return schema_as_node(elements, valid_set_keys);
            }
            resolve_at(index + 1, segments, elements, valid_set_keys)
        }
        SchemaShape::Struct { fields } => {
            if fields.is_empty() {
                return Err(fail("schema child nodes empty".to_string()));
            }
            match fields.iter().find(|(name, _)| *name == collection_key) {
                Some((_, field_schema)) => {
                    if index == last {
                        schema_as_node(field_schema, valid_set_keys)
                    } else {
                        resolve_at(index + 1, segments, field_schema, valid_set_keys)
                    }
                }
                None => Err(fail(format!(
                    "schema for struct field '{collection_key}' not found"
                ))),
            }
        }
        // Leaf shapes: no children to descend into; return the node itself.
        SchemaShape::Scalar | SchemaShape::Any | SchemaShape::Enum { .. } => Ok(node.clone()),
    }
}

/// Reduces a schema to a single node: `Node` clones, `Set` yields its default
/// node (or the first node when no default key is set).
fn schema_as_node(
    schema: &Schema,
    valid_set_keys: &mut Vec<String>,
) -> Result<SchemaNode, SchemaError> {
    match schema {
        Schema::Node(node) => Ok(node.clone()),
        Schema::Set(set) => {
            if let Some(default_key) = &set.default_key {
                if let Some((_, node)) = set.nodes.iter().find(|(key, _)| key == default_key) {
                    valid_set_keys.push(default_key.clone());
                    return Ok(node.clone());
                }
            }
            match set.nodes.first() {
                Some((key, node)) => {
                    valid_set_keys.push(key.clone());
                    Ok(node.clone())
                }
                None => Err(SchemaError::SchemaPathError {
                    message: "no schema nodes found".to_string(),
                    path: render_path(&[]),
                }),
            }
        }
    }
}
