//! Database subdomain application use cases (port of
//! go-metadatamodel/database): extract the columns of one database table
//! collection from a metadata model (`get_column_fields`), and get/set/delete
//! field values in source data through those columns (`FieldValue`).
//!
//! The `db:*` properties drive the mapping:
//! - `db:table_collection_uid` or (`db:join_depth` + `db:table_collection_name`)
//!   select which table collection's fields are collected,
//! - `db:column_name` names the column a field maps to,
//! - each field's `group:path_key` template is post-processed (core's
//!   `data_path`) into the actual JSONPath to the field's data.

use dyn_rt_manipulation::application::DynObject;
use dyn_rt_path::{AccessStrategy, PathSegment, PathSelector, RecursiveDescentSegments};
use dyn_rt_reflect::{Reflect, ReflectKind};
use dyn_rt_schema::Schema;
use dyn_rt_shared::Value;

use super::domain::ColumnFields;
use crate::core::application::{
    cardinality_of, data_path, fields_of, is_group, parse_template, path_key_of, prop, props_of,
    read_order_of,
};
use crate::core::domain::{FIELDS_KEY, MetadataModel, props};
use crate::error::MetadatamodelError;
use crate::fieldcolumns::application::PropMatch;

// ---------------------------------------------------------------------------
// get_column_fields
// ---------------------------------------------------------------------------

/// Options for [`get_column_fields`] (go's `GetColumnFields` builder fields).
#[derive(Debug, Clone, Default)]
pub struct GetColumnFieldsOptions {
    /// Select by `db:table_collection_uid`.
    pub table_collection_uid: Option<String>,
    /// Select by `db:join_depth` (requires `table_collection_name` too).
    pub join_depth: Option<i64>,
    /// Select by `db:table_collection_name` (requires `join_depth` too).
    pub table_collection_name: Option<String>,
    /// Skip a field/group whose props match one of the entries (collection
    /// only — descent into the group's fields still happens).
    pub skip: Option<PropMatch>,
    /// Collect only field/groups whose props match one of the entries.
    pub add: Option<PropMatch>,
}

/// Extracts the database columns of one table collection from the metadata
/// model (go's `GetColumnFields.Get`). Requires `table_collection_uid` OR
/// both `join_depth` and `table_collection_name`.
pub fn get_column_fields(
    model: &MetadataModel,
    options: &GetColumnFieldsOptions,
) -> Result<ColumnFields, MetadatamodelError> {
    if options.table_collection_uid.is_none()
        && (options.join_depth.is_none() || options.table_collection_name.is_none())
    {
        return Err(MetadatamodelError::GetColumnFields {
            message:
                "db:table_collection_uid or db:join_depth and db:table_collection_name is required"
                    .to_string(),
        });
    }

    let mut out = ColumnFields::new();
    walk_fields(model, options, &mut out)?;
    Ok(out)
}

/// Recursive walk over every field/group of `model` in read order (go's
/// `iter.ForEach` as used by `GetColumnFields.Get`).
fn walk_fields(
    model: &MetadataModel,
    options: &GetColumnFieldsOptions,
    out: &mut ColumnFields,
) -> Result<(), MetadatamodelError> {
    let fields = fields_of(model)?;
    let order = read_order_of(model)?;
    let fields_obj = fields
        .as_object()
        .ok_or_else(|| MetadatamodelError::InvalidModel {
            message: format!("{FIELDS_KEY}[0] is not an object"),
        })?;

    for key in &order {
        let entry = fields_obj
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, entry)| entry)
            .ok_or_else(|| MetadatamodelError::InvalidModel {
                message: format!("field {key:?} not found in {FIELDS_KEY}[0]"),
            })?;
        let props = props_of(entry)?;

        // skip / add affect collection only — descent into groups continues
        let skipped = options
            .skip
            .as_ref()
            .is_some_and(|skip| skip.is_valid() && skip.first_match(props));
        let not_added = options
            .add
            .as_ref()
            .is_some_and(|add| add.is_valid() && !add.first_match(props));
        if skipped || not_added {
            if is_group(props) {
                walk_fields(props, options, out)?;
            }
            continue;
        }

        // the table collection criterion
        let matches = if let Some(uid) = &options.table_collection_uid {
            prop(props, props::DB_TABLE_COLLECTION_UID).and_then(Value::as_str)
                == Some(uid.as_str())
        } else {
            let depth_matches = prop(props, props::DB_JOIN_DEPTH)
                .and_then(Value::as_f64)
                .is_some_and(|depth| depth as i64 == options.join_depth.unwrap_or(i64::MIN));
            let name_matches = prop(props, props::DB_TABLE_COLLECTION_NAME).and_then(Value::as_str)
                == options.table_collection_name.as_deref();
            depth_matches && name_matches
        };
        if !matches {
            // non-matching subtree: do not descend into it
            continue;
        }

        if is_group(props) {
            walk_fields(props, options, out)?;
            continue;
        }

        let column_name = prop(props, props::DB_COLUMN_NAME)
            .and_then(Value::as_str)
            .unwrap_or("");
        if column_name.is_empty() {
            return Err(MetadatamodelError::GetColumnFields {
                message: format!("db:column_name not found in field/group property {key:?}"),
            });
        }
        if out.get(column_name).is_some() {
            return Err(MetadatamodelError::GetColumnFields {
                message: format!("duplicate db:column_name '{column_name}' found"),
            });
        }
        out.read_order.push(column_name.to_string());
        out.fields.push((column_name.to_string(), props.clone()));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// field value (get / set / delete in source data through the columns)
// ---------------------------------------------------------------------------

/// Builds the JSONPath to a column's data in the source: the field's
/// `group:path_key` template post-processed by core's `data_path`, with
/// `suffix_path` appended (go's `FieldValue.getJsonPathToValue`).
///
/// When `source_is_array` is true the source root is itself a sequence, so
/// the first index addresses the root entry and the remaining indices the
/// template's wildcards.
pub fn field_value_path(
    column_fields: &ColumnFields,
    column_name: &str,
    suffix_path: &str,
    indices: &[usize],
    source_is_array: bool,
) -> Result<RecursiveDescentSegments, MetadatamodelError> {
    if column_name.is_empty() {
        return Err(MetadatamodelError::FieldValue {
            message: "column field name is empty".to_string(),
        });
    }
    let props = column_fields
        .get(column_name)
        .ok_or_else(|| MetadatamodelError::FieldValue {
            message: format!("field with db:column_name '{column_name}' not found"),
        })?;
    let template = path_key_of(props)?;
    let cardinality = cardinality_of(props);

    let (root_index, wildcard_indices) = if source_is_array {
        (Some(indices.first().copied().unwrap_or(0)), &indices[1..])
    } else {
        (None, indices)
    };

    let mut path = data_path(template, wildcard_indices, cardinality)?;
    if !suffix_path.is_empty() {
        path = format!("{path}.{suffix_path}");
    }
    let mut segments = parse_template(&path)?;

    if let Some(index) = root_index {
        // source root is a sequence: address the root entry right after `$`
        if let Some(first_group) = segments.0.first_mut() {
            first_group.0.insert(
                1,
                PathSegment {
                    access_strategy: AccessStrategy::Positional,
                    selector: PathSelector::Index(index as isize),
                },
            );
        }
    }
    Ok(segments)
}

/// Get/set/delete field values in `source` using the extracted columns (go's
/// `FieldValue`). `source_is_array` should be true when the source root is a
/// sequence — [`FieldValue::new`] detects it automatically.
pub struct FieldValue<'a> {
    /// The source data to manipulate, wrapped in the DynObject entrypoint.
    pub obj: DynObject<'a>,
    /// The extracted database columns (from [`get_column_fields`]).
    pub column_fields: &'a ColumnFields,
    /// Whether the source root is a sequence (auto-detected in
    /// [`FieldValue::new`], like go's `SetSourceData`).
    pub source_is_array: bool,
}

impl<'a> FieldValue<'a> {
    pub fn new(source: DynObject<'a>, column_fields: &'a ColumnFields) -> Self {
        let source_is_array = matches!(source.source().reflect_ref().kind(), ReflectKind::Sequence);
        FieldValue {
            obj: source,
            column_fields,
            source_is_array,
        }
    }

    fn path_segments(
        &self,
        column_name: &str,
        suffix_path: &str,
        indices: &[usize],
    ) -> Result<RecursiveDescentSegments, MetadatamodelError> {
        field_value_path(
            self.column_fields,
            column_name,
            suffix_path,
            indices,
            self.source_is_array,
        )
    }

    /// Gets the value(s) of column `column_name` (go's `FieldValue.Get`).
    /// Returns one entry per path match.
    pub fn get<'b>(
        &'b self,
        column_name: &str,
        suffix_path: &str,
        indices: &[usize],
    ) -> Result<Vec<&'b dyn Reflect>, MetadatamodelError> {
        let segments = self.path_segments(column_name, suffix_path, indices)?;
        self.obj
            .get(&segments)
            .map_err(|err| MetadatamodelError::Manipulation {
                message: err.to_string(),
            })
    }

    /// Sets `value` at column `column_name` (go's `FieldValue.Set`). A schema
    /// guides coercion and creation of missing intermediate structures.
    /// Returns how many values were set.
    pub fn set(
        &mut self,
        column_name: &str,
        value: &dyn Reflect,
        suffix_path: &str,
        indices: &[usize],
        schema: Option<&Schema>,
    ) -> Result<usize, MetadatamodelError> {
        let segments = self.path_segments(column_name, suffix_path, indices)?;
        self.obj
            .set_with_schema(&segments, value, schema.cloned())
            .map_err(|err| MetadatamodelError::Manipulation {
                message: err.to_string(),
            })
    }

    /// Deletes the value(s) of column `column_name` (go's `FieldValue.Delete`).
    /// Returns how many values were deleted.
    pub fn delete(
        &mut self,
        column_name: &str,
        suffix_path: &str,
        indices: &[usize],
    ) -> Result<usize, MetadatamodelError> {
        let segments = self.path_segments(column_name, suffix_path, indices)?;
        self.obj
            .delete(&segments)
            .map_err(|err| MetadatamodelError::Manipulation {
                message: err.to_string(),
            })
    }
}
