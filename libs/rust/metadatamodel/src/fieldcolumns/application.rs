//! fieldcolumns application: extract the metadata model's fields as flat
//! 2D table columns (port of go-metadatamodel/fieldcolumns extraction.go),
//! plus the skip operation (column_fields.go).
//!
//! Walk order is `group:read_order_of_fields` (default: the fields object's
//! key order — Rust refinement, go errors when the prop is absent). A group
//! is recursed into, unless `extract:as_single_field` is set or it expands
//! into separate columns (`view:values_in_separate_columns` with
//! `view:max_values_in_separate_columns` > 0 and no nested groups). A field
//! with that view prop expands into one column per value index.

use dyn_rt_path::{RegexPathTokenizer, parse_path};
use dyn_rt_schema::{Schema, schema_at_path};
use dyn_rt_shared::Value;

use super::domain::{ColumnField, ColumnFields, FieldColumnPosition};
use crate::core::application::{
    cardinality_of, contains_nested_groups, data_path, fields_of, is_group, merge_props, name_of,
    path_key_of, prop, props_of, read_order_of, set_prop,
};
use crate::core::domain::{Cardinality, FIELDS_KEY, MetadataModel, props};
use crate::error::MetadatamodelError;

// ---------------------------------------------------------------------------
// property matching (go's FieldGroupPropertiesMatch, DeepEqual semantics)
// ---------------------------------------------------------------------------

/// A set of property entries that must all be present and deep-equal for a
/// match. `matching_props` returns the matched subset (merged into nested
/// fields by the extraction).
#[derive(Debug, Clone, Default)]
pub struct PropMatch {
    pub entries: Vec<(String, Value)>,
}

impl PropMatch {
    pub fn is_valid(&self) -> bool {
        !self.entries.is_empty()
    }

    /// Whether every entry is present in `props` and deep-equal.
    pub fn first_match(&self, props: &Value) -> bool {
        self.entries.iter().all(|(key, expected)| {
            prop(props, key).is_some_and(|got| dyn_rt_manipulation::are_equal(got, expected))
        })
    }

    /// The subset of `self.entries` present and equal in `props`.
    pub fn matching_props(&self, props: &Value) -> Value {
        let mut out = Vec::new();
        for (key, expected) in &self.entries {
            if prop(props, key).is_some_and(|got| dyn_rt_manipulation::are_equal(got, expected)) {
                out.push((key.clone(), expected.clone()));
            }
        }
        Value::Object(out)
    }
}

// ---------------------------------------------------------------------------
// extraction
// ---------------------------------------------------------------------------

/// Options for [`extract`].
#[derive(Debug, Clone, Default)]
pub struct ExtractOptions {
    /// Schema of the data the model describes; when set, each column carries
    /// the schema at its data path.
    pub schema: Option<Schema>,
    /// When valid, matching fields/groups get the match merged into their
    /// nested fields (go's skip/add propagation).
    pub skip: Option<PropMatch>,
    pub add: Option<PropMatch>,
}

/// Extracts the model's fields as columns. `model` is the root group's props
/// object.
pub fn extract(
    model: &MetadataModel,
    options: &ExtractOptions,
) -> Result<ColumnFields, MetadatamodelError> {
    let mut out = ColumnFields::default();
    let mut matching = Value::Object(Vec::new());
    let mut next_position: Option<FieldColumnPosition> = None;
    recursive_extract(model, &mut matching, &mut next_position, &mut out, options)?;
    Ok(out)
}

fn recursive_extract(
    group: &Value,
    matching: &mut Value,
    next_position: &mut Option<FieldColumnPosition>,
    out: &mut ColumnFields,
    options: &ExtractOptions,
) -> Result<(), MetadatamodelError> {
    let fields = fields_of(group)?;
    let read_order = read_order_of(group)?;
    let fields_obj = fields
        .as_object()
        .ok_or_else(|| MetadatamodelError::InvalidModel {
            message: format!("{FIELDS_KEY}[0] is not an object"),
        })?;

    for key_suffix in &read_order {
        let entry = fields_obj
            .iter()
            .find(|(key, _)| key == key_suffix)
            .map(|(_, entry)| entry)
            .ok_or_else(|| MetadatamodelError::InvalidModel {
                message: format!("field {key_suffix:?} not found in {FIELDS_KEY}"),
            })?;
        let fg_props = props_of(entry)?;

        // merge skip/add matches into the inherited matching props
        let mut fg_matching = matching.clone();
        if let Some(skip) = &options.skip
            && skip.is_valid()
        {
            merge_props(&mut fg_matching, &skip.matching_props(fg_props));
        }
        if let Some(add) = &options.add
            && add.is_valid()
        {
            merge_props(&mut fg_matching, &add.matching_props(fg_props));
        }

        // position override from view:column_position, else inherited
        if let Some(position) = extract_column_position(fg_props) {
            *next_position = Some(position);
        }

        let path_key = path_key_of(fg_props)?.to_string();

        if is_group(fg_props) {
            let as_single = prop(fg_props, props::EXTRACT_AS_SINGLE_FIELD)
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let max_columns = max_flat_columns(fg_props)?;
            if !as_single && max_columns > 0 && !contains_nested_groups(fg_props)? {
                // group expanded into separate columns: each sub-field
                // becomes a column per value index
                extract_group_as_columns(
                    fg_props,
                    &path_key,
                    &fg_matching,
                    next_position,
                    out,
                    options,
                    max_columns,
                )?;
                continue;
            }

            // recurse into the group
            recursive_extract(fg_props, &mut fg_matching, next_position, out, options)?;
            continue;
        }

        // plain field (or group as single field)
        let max_columns = max_flat_columns(fg_props)?;
        if max_columns > 0 {
            // field expanded into separate columns, one per value index
            for column_index in 0..max_columns as usize {
                let mut new_field = fg_props.clone();
                merge_props(&mut new_field, &fg_matching);
                add_flat_column_context(&mut new_field, column_index);
                let position = FieldColumnPosition {
                    path_key: path_key.clone(),
                    field_in_separate_columns: true,
                    field_header_index: column_index,
                    ..Default::default()
                };
                append_field(&new_field, &position, out, options)?;
                // one reposition instruction per position prop: consumed by
                // the first column appended after it (go flows it onward;
                // recording once avoids cascade drift)
                if let Some(mut flowing) = next_position.take() {
                    set_reposition(&mut flowing, out);
                }
            }
            continue;
        }

        // plain field, single column
        let mut new_field = fg_props.clone();
        merge_props(&mut new_field, &fg_matching);
        let position = FieldColumnPosition {
            path_key: path_key.clone(),
            ..Default::default()
        };
        append_field(&new_field, &position, out, options)?;
        if let Some(mut flowing) = next_position.take() {
            set_reposition(&mut flowing, out);
        }
    }

    Ok(())
}

/// A non-nested group with separate columns: each sub-field becomes one
/// column per value index, keyed `parent[i].suffix`.
fn extract_group_as_columns(
    fg_props: &Value,
    path_key: &str,
    fg_matching: &Value,
    next_position: &mut Option<FieldColumnPosition>,
    out: &mut ColumnFields,
    options: &ExtractOptions,
    max_columns: i64,
) -> Result<(), MetadatamodelError> {
    let nested_fields = fields_of(fg_props)?;
    let nested_order = read_order_of(fg_props)?;
    let nested_obj = nested_fields
        .as_object()
        .ok_or_else(|| MetadatamodelError::InvalidModel {
            message: format!("{FIELDS_KEY}[0] is not an object"),
        })?;
    for column_index in 0..max_columns as usize {
        for nested_suffix in &nested_order {
            let nested_entry = nested_obj
                .iter()
                .find(|(key, _)| key == nested_suffix)
                .map(|(_, entry)| entry)
                .ok_or_else(|| MetadatamodelError::InvalidModel {
                    message: format!("nested field {nested_suffix:?} not found"),
                })?;
            let nested_props = props_of(nested_entry)?;
            let mut new_field = nested_props.clone();
            merge_props(&mut new_field, fg_matching);
            add_flat_column_context(&mut new_field, column_index);
            let nested_key = path_key_of(nested_props)?.to_string();
            let position = FieldColumnPosition {
                path_key: nested_key.clone(),
                group_in_separate_columns: true,
                group_header_index: column_index,
                group_parent_path_key: path_key.to_string(),
                field_key_suffix: nested_suffix.clone(),
                ..Default::default()
            };
            append_field(&new_field, &position, out, options)?;
            if let Some(mut flowing) = next_position.take() {
                set_reposition(&mut flowing, out);
            }
        }
    }
    Ok(())
}

/// Records `position` as a reposition instruction with source = the column
/// just appended (go's setRepositionForFieldColumn).
fn set_reposition(position: &mut FieldColumnPosition, out: &mut ColumnFields) {
    position.source_index = out.read_order.len().saturating_sub(1);
    out.reposition.push(position.clone());
}

/// Maximum number of separate columns for a field/group: the
/// `view:max_values_in_separate_columns` value when
/// `view:values_in_separate_columns` is true (groups containing nested
/// groups force -1), else -1. A `single` cardinality caps it at 1.
pub fn max_flat_columns(fg_props: &Value) -> Result<i64, MetadatamodelError> {
    let mut max_columns = -1i64;
    let in_separate = prop(fg_props, props::VIEW_VALUES_IN_SEPARATE_COLUMNS)
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if in_separate {
        if is_group(fg_props) && contains_nested_groups(fg_props)? {
            return Ok(-1);
        }
        if let Some(value) = prop(fg_props, props::VIEW_MAX_VALUES_IN_SEPARATE_COLUMNS) {
            max_columns = value
                .as_f64()
                .ok_or_else(|| MetadatamodelError::InvalidModel {
                    message: format!(
                        "{:?} is not a number",
                        props::VIEW_MAX_VALUES_IN_SEPARATE_COLUMNS
                    ),
                })? as i64;
        }
        if cardinality_of(fg_props) == Cardinality::Single && max_columns > 1 {
            max_columns = 1;
        }
    }
    Ok(max_columns)
}

/// Reads `view:column_position` (`{ group:path_key, view:position_before,
/// view:separate_columns_header_index }`).
fn extract_column_position(fg_props: &Value) -> Option<FieldColumnPosition> {
    let value = prop(fg_props, props::VIEW_COLUMN_POSITION)?;
    let obj = value.as_object()?;
    let get = |key: &str| obj.iter().find(|(k, _)| k == key).map(|(_, v)| v);
    let path_key = get(props::GROUP_PATH_KEY)?.as_str()?.to_string();
    Some(FieldColumnPosition {
        path_key,
        position_before: get(props::VIEW_POSITION_BEFORE)
            .and_then(Value::as_bool)
            .unwrap_or(false),
        field_header_index: get(props::VIEW_SEPARATE_COLUMNS_HEADER_INDEX)
            .and_then(Value::as_f64)
            .map(|v| v as usize)
            .unwrap_or(0),
        ..Default::default()
    })
}

/// Stamps a separate-column field copy with its header index and name (go's
/// addFlatColumnContext): the `[*]` in the header format is replaced by the
/// 1-based column number, else `"<name> <n>"`.
fn add_flat_column_context(field: &mut Value, column_index: usize) {
    // hoist the reads that need &field before the mutable object borrow
    let header_format = field
        .get(props::VIEW_SEPARATE_COLUMNS_HEADER_FORMAT)
        .and_then(Value::as_str)
        .map(|s| s.to_string());
    let base_name = name_of(field, "");
    set_prop(
        field,
        props::VIEW_SEPARATE_COLUMNS_HEADER_INDEX,
        Value::from(column_index as i64),
    );
    let new_name = match header_format {
        Some(format) if !format.is_empty() => {
            format.replace("[*]", &(column_index + 1).to_string())
        }
        _ => format!("{} {}", base_name, column_index + 1),
    };
    set_prop(field, props::GROUP_NAME, Value::from(new_name));
}

fn append_field(
    field: &Value,
    position: &FieldColumnPosition,
    out: &mut ColumnFields,
    options: &ExtractOptions,
) -> Result<(), MetadatamodelError> {
    // per-column schema: the schema node at the field's data path
    let schema = options.schema.as_ref().and_then(|schema| {
        let cardinality = cardinality_of(field);
        let template = path_key_of(field).ok()?;
        let path = data_path(template, &[], cardinality).ok()?;
        let segments = parse_path(&RegexPathTokenizer, &path);
        let group = segments.0.into_iter().next()?;
        schema_at_path(&group, schema)
            .ok()
            .map(|result| Schema::Node(result.node))
    });
    let column = ColumnField {
        props: field.clone(),
        schema,
        index_in_read_order: out.read_order.len(),
        index_in_repositioned: 0,
        skip: false,
    };
    out.fields.push((position.column_key(), column));
    out.read_order.push(position.clone());
    let index = out.read_order.len() - 1;
    out.current.push(index);
    Ok(())
}

// ---------------------------------------------------------------------------
// skip (column_fields.go)
// ---------------------------------------------------------------------------

impl ColumnFields {
    /// Marks columns to skip: a valid skip-match skips the column; a valid
    /// add-match keeps it only when it matches.
    pub fn skip(&mut self, skip: Option<&PropMatch>, add: Option<&PropMatch>) {
        for (_, field) in &mut self.fields {
            field.skip = false;
            if let Some(skip) = skip.filter(|m| m.is_valid())
                && skip.first_match(&field.props)
            {
                field.skip = true;
            }
            if let Some(add) = add.filter(|m| m.is_valid())
                && !add.first_match(&field.props)
            {
                field.skip = true;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// group column indexes (group_column_indexes.go) — used by flattener and
// unflattener to know which columns belong to each group
// ---------------------------------------------------------------------------

/// Fast lookup index identifying which columns to read at a group (go's
/// `GroupColumnIndexes`).
#[derive(Debug, Clone, Default)]
pub struct GroupColumnIndexes {
    /// Primary key columns (indexes into the unskipped current read order).
    pub primary: Vec<usize>,
    /// All columns of the group (same indexing).
    pub all: Vec<usize>,
}

/// Retrieves the column indexes (primary and all) of `group` within
/// `column_fields` (go's `GroupsColumnsIndexesRetrieval.Get`).
pub fn group_column_indexes(
    column_fields: &ColumnFields,
    group: &MetadataModel,
) -> Result<GroupColumnIndexes, MetadatamodelError> {
    let mut indexes = GroupColumnIndexes::default();
    collect_group_column_indexes(column_fields, group, true, false, &mut indexes)?;
    Ok(indexes)
}

/// The leaf positions a field (or group-as-separate-columns) contributes:
/// one per separate column when `max_flat_columns > 0`, else a single one.
fn leaf_positions(path_key: &str, max_columns: i64) -> Vec<FieldColumnPosition> {
    let mut positions = Vec::new();
    if max_columns > 0 {
        for column_index in 0..max_columns as usize {
            positions.push(FieldColumnPosition {
                path_key: path_key.to_string(),
                field_in_separate_columns: true,
                field_header_index: column_index,
                ..Default::default()
            });
        }
    } else {
        positions.push(FieldColumnPosition {
            path_key: path_key.to_string(),
            ..Default::default()
        });
    }
    positions
}

fn append_column_index(
    column_fields: &ColumnFields,
    position: &FieldColumnPosition,
    is_primary: bool,
    is_all: bool,
    indexes: &mut GroupColumnIndexes,
) -> Result<(), MetadatamodelError> {
    let field = column_fields
        .get_by_key(&position.column_key())
        .ok_or_else(|| MetadatamodelError::InvalidModel {
            message: format!(
                "column '{}' not found in column fields",
                position.column_key()
            ),
        })?;
    if is_all {
        if let Some(index) = column_fields.index_in_unskipped(field.index_in_read_order) {
            indexes.all.push(index);
        }
    }
    if is_primary {
        if let Some(index) = column_fields.index_in_unskipped(field.index_in_read_order) {
            indexes.primary.push(index);
        }
    }
    Ok(())
}

fn collect_group_column_indexes(
    column_fields: &ColumnFields,
    group: &MetadataModel,
    is_all: bool,
    force_primary: bool,
    indexes: &mut GroupColumnIndexes,
) -> Result<(), MetadatamodelError> {
    let order = read_order_of(group)?;
    let fields = fields_of(group)?;
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
        let field_is_primary = force_primary
            || prop(props, props::GROUP_IS_PRIMARY_KEY)
                .and_then(Value::as_bool)
                .unwrap_or(false);
        let path_key = path_key_of(props)?.to_string();

        if is_group(props) {
            let as_single = prop(props, props::EXTRACT_AS_SINGLE_FIELD)
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let max_columns = max_flat_columns(props)?;
            if !as_single && max_columns > 0 && !contains_nested_groups(props)? {
                // group-as-separate-columns: leaves per nested field per column
                let nested_order = read_order_of(props)?;
                let nested_fields = fields_of(props)?;
                let nested_obj =
                    nested_fields
                        .as_object()
                        .ok_or_else(|| MetadatamodelError::InvalidModel {
                            message: format!("{FIELDS_KEY}[0] is not an object"),
                        })?;
                for column_index in 0..max_columns as usize {
                    for nested_suffix in &nested_order {
                        let nested_entry = nested_obj
                            .iter()
                            .find(|(k, _)| k == nested_suffix)
                            .map(|(_, e)| e)
                            .ok_or_else(|| MetadatamodelError::InvalidModel {
                                message: format!("nested field {nested_suffix:?} not found"),
                            })?;
                        let nested_key = path_key_of(props_of(nested_entry)?)?.to_string();
                        let position = FieldColumnPosition {
                            path_key: nested_key,
                            group_in_separate_columns: true,
                            group_header_index: column_index,
                            group_parent_path_key: path_key.clone(),
                            field_key_suffix: nested_suffix.clone(),
                            ..Default::default()
                        };
                        append_column_index(
                            column_fields,
                            &position,
                            field_is_primary,
                            true,
                            indexes,
                        )?;
                    }
                }
                continue;
            }
            if as_single {
                // processed as a plain field below
            } else {
                // nested group: contributes no columns itself; its primary
                // keys bubble up when the group is primary
                if field_is_primary {
                    collect_group_column_indexes(column_fields, props, false, true, indexes)?;
                }
                continue;
            }
        }

        // plain field (or group as single field)
        let max_columns = max_flat_columns(props)?;
        for position in leaf_positions(&path_key, max_columns) {
            append_column_index(column_fields, &position, field_is_primary, is_all, indexes)?;
        }
    }
    Ok(())
}
