//! Flattener subdomain application use case (port of go-metadatamodel/flattener):
//! convert a deeply nested mix of collections (e.g. an array of objects)
//! into a 2D table, using the fieldcolumns extraction of the metadata model
//! for column order/skip, and optionally write the table to a destination
//! 2D collection (`write_to_destination`).

use dyn_rt_json_manipulation::application::{get, set};
use dyn_rt_json_path::RecursiveDescentSegments;
use dyn_rt_json_schema::Schema;
use dyn_rt_reflect::{Reflect, ReflectRef};
use dyn_rt_shared::Value;

use super::domain::{ConversionNode, FlattenedRow, FlattenedTable};
use crate::core::application::{
    contains_nested_groups, data_path_model_aware, fields_of, is_group, parse_template,
    path_key_of, prop, props_of, read_order_of,
};
use crate::core::domain::{FIELDS_KEY, MetadataModel, props};
use crate::error::MetadatamodelError;
use crate::fieldcolumns::{ColumnFields, FieldColumnPosition, extract, max_flat_columns};

/// Converts objects into a 2D table (go's `Flattener`).
pub struct Flattener<'a> {
    /// The metadata model describing the source data.
    pub model: &'a MetadataModel,
    /// Extracted (and repositioned) column fields driving column order/skip.
    pub column_fields: ColumnFields,
    tree: ConversionNode,
    result: FlattenedTable,
}

impl<'a> Flattener<'a> {
    /// Creates a flattener, extracting the default column fields from the
    /// model (go's `NewFlattener` + default extraction in `Flatten`).
    pub fn new(model: &'a MetadataModel) -> Result<Self, MetadatamodelError> {
        let mut column_fields = extract(model, &Default::default())?;
        column_fields.reposition();
        column_fields.update_index_in_repositioned();
        Self::with_column_fields(model, column_fields)
    }

    /// Creates a flattener with caller-provided column fields (go's
    /// `WithColumnFields`).
    pub fn with_column_fields(
        model: &'a MetadataModel,
        column_fields: ColumnFields,
    ) -> Result<Self, MetadatamodelError> {
        let tree = build_conversion_tree(model, "$", &column_fields, true)?;
        Ok(Flattener {
            model,
            column_fields,
            tree,
            result: Vec::new(),
        })
    }

    /// The current flattened table (go's `GetResult`).
    pub fn result(&self) -> &FlattenedTable {
        &self.result
    }

    /// Clears the current result (go's `Reset`).
    pub fn reset(&mut self) {
        self.result = Vec::new();
    }

    /// Flattens `source` and appends the rows to the result. When the source
    /// root is a sequence, each element is flattened individually (go's
    /// `Flatten`).
    pub fn flatten(&mut self, source: &dyn Reflect) -> Result<(), MetadatamodelError> {
        if matches!(
            source.reflect_ref().kind(),
            dyn_rt_reflect::ReflectKind::Sequence
        ) {
            let sequence = match source.reflect_ref() {
                ReflectRef::Sequence(seq) => seq,
                _ => unreachable!(),
            };
            for index in 0..sequence.reflect_length() {
                let Some(element) = sequence.reflect_get(index) else {
                    continue;
                };
                let rows = recursive_convert(&self.tree, self.model, element, &[], Vec::new())?;
                self.result.extend(rows);
            }
        } else {
            let rows = recursive_convert(&self.tree, self.model, source, &[], Vec::new())?;
            self.result.extend(rows);
        }
        Ok(())
    }

    /// Writes the current result to a destination 2D collection at
    /// `$[row][col]`, honoring the unskipped current column read order
    /// (go's `WriteToDestination`).
    pub fn write_to_destination(
        &self,
        destination: &mut dyn Reflect,
        schema: Option<&Schema>,
    ) -> Result<(), MetadatamodelError> {
        let read_order = self.column_fields.current_read_order();
        for (row_index, row) in self.result.iter().enumerate() {
            for (dest_col, source_col) in read_order.iter().enumerate() {
                let Some(source_col) = row.get(*source_col) else {
                    return Err(MetadatamodelError::Flatten {
                        message: format!(
                            "source column index {source_col} out of bounds (row {row_index}, row length {})",
                            row.len()
                        ),
                    });
                };
                let target = parse_template(&format!("$[{row_index}][{dest_col}]"))?;
                set(destination, &target, source_col, schema).map_err(|err| {
                    MetadatamodelError::Manipulation {
                        message: format!("write failed at row {row_index} col {dest_col}: {err}"),
                    }
                })?;
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// conversion tree (go's recursiveInitRecursiveIndexTree)
// ---------------------------------------------------------------------------

/// Builds the conversion tree for `group` (root when `is_root`). A group
/// with no usable fields is an error at the root and skipped when nested
/// (go's `ErrNoGroupFields`).
fn build_conversion_tree(
    group: &MetadataModel,
    group_path_key: &str,
    column_fields: &ColumnFields,
    is_root: bool,
) -> Result<ConversionNode, MetadatamodelError> {
    let mut node = ConversionNode {
        position: FieldColumnPosition {
            path_key: group_path_key.to_string(),
            ..Default::default()
        },
        children: Vec::new(),
    };

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
        let path_key = path_key_of(props)?.to_string();

        if is_group(props) {
            let as_single = prop(props, props::EXTRACT_AS_SINGLE_FIELD)
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let max_columns = max_flat_columns(props)?;
            if !as_single && max_columns > 0 && !contains_nested_groups(props)? {
                // group-as-separate-columns: one leaf per nested field per column
                let nested_order = read_order_of(props)?;
                let nested_obj = fields_of(props)?.as_object().ok_or_else(|| {
                    MetadatamodelError::InvalidModel {
                        message: format!("{FIELDS_KEY}[0] is not an object"),
                    }
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
                        push_leaf(&mut node, position, column_fields);
                    }
                }
                continue;
            }

            // nested group: a child node with its own children
            match build_conversion_tree(props, &path_key, column_fields, false) {
                Ok(child) => node.children.push(child),
                Err(err) if !is_root => {
                    // go's ErrNoGroupFields: skip empty nested groups
                    if !err.to_string().contains("no group fields") {
                        return Err(err);
                    }
                }
                Err(err) => return Err(err),
            }
            continue;
        }

        // plain field
        let max_columns = max_flat_columns(props)?;
        if max_columns > 0 {
            for column_index in 0..max_columns as usize {
                let position = FieldColumnPosition {
                    path_key: path_key.clone(),
                    field_in_separate_columns: true,
                    field_header_index: column_index,
                    ..Default::default()
                };
                push_leaf(&mut node, position, column_fields);
            }
        } else {
            let position = FieldColumnPosition {
                path_key: path_key.clone(),
                ..Default::default()
            };
            push_leaf(&mut node, position, column_fields);
        }
    }

    if node.children.is_empty() {
        return Err(MetadatamodelError::Flatten {
            message: "no group fields to extract found".to_string(),
        });
    }
    Ok(node)
}

/// Adds a leaf child when the position maps to an unskipped column.
fn push_leaf(
    node: &mut ConversionNode,
    position: FieldColumnPosition,
    column_fields: &ColumnFields,
) {
    if let Some(field) = column_fields.get_by_key(&position.column_key())
        && !field.skip
    {
        node.children.push(ConversionNode {
            position,
            children: Vec::new(),
        });
    }
}

// ---------------------------------------------------------------------------
// recursive convert (go's recursiveConvert)
// ---------------------------------------------------------------------------

/// The data path a leaf reads: `linear_indexes` address the enclosing group
/// arrays; separate-column leaves additionally index the field's own array
/// with their header index; plain leaves read the field's whole value.
/// Cardinality is model-aware: `single` fields/groups drop their `[*]`.
fn leaf_data_path(
    position: &FieldColumnPosition,
    model: &MetadataModel,
    linear_indexes: &[usize],
) -> Result<RecursiveDescentSegments, MetadatamodelError> {
    if position.group_in_separate_columns {
        // parent group array indexed by the header index, then the bare suffix
        let mut indexes = linear_indexes.to_vec();
        indexes.push(position.group_header_index);
        let parent = data_path_model_aware(&position.group_parent_path_key, model, &indexes, true)?;
        return parse_template(&format!("{parent}.{}", position.field_key_suffix));
    }
    if position.field_in_separate_columns {
        // the field's own array indexed by the column header index
        let mut indexes = linear_indexes.to_vec();
        indexes.push(position.field_header_index);
        return parse_template(&data_path_model_aware(
            &position.path_key,
            model,
            &indexes,
            true,
        )?);
    }
    // plain leaf: read the whole value (last wildcard dropped)
    parse_template(&data_path_model_aware(
        &position.path_key,
        model,
        linear_indexes,
        false,
    )?)
}

/// The data path a group node branches on: the whole group value (last
/// wildcard dropped), with enclosing group arrays indexed.
fn group_data_path(
    group_template: &str,
    model: &MetadataModel,
    linear_indexes: &[usize],
) -> Result<RecursiveDescentSegments, MetadatamodelError> {
    parse_template(&data_path_model_aware(
        group_template,
        model,
        linear_indexes,
        false,
    )?)
}

/// Converts `source` into rows according to `node`, branching per array
/// element of nested groups (Cartesian product across group arrays).
fn recursive_convert(
    node: &ConversionNode,
    model: &MetadataModel,
    source: &dyn Reflect,
    linear_indexes: &[usize],
    incoming_rows: FlattenedTable,
) -> Result<FlattenedTable, MetadatamodelError> {
    let mut current_rows = incoming_rows;
    if current_rows.is_empty() {
        current_rows.push(Vec::new());
    }

    for child in &node.children {
        // resolve the child's own value at the current position
        let child_segments = if child.children.is_empty() {
            leaf_data_path(&child.position, model, linear_indexes)?
        } else {
            // group node: read the WHOLE group value (trailing [*] trimmed);
            // branching per array element happens below on the actual kind
            group_data_path(&child.position.path_key, model, linear_indexes)?
        };
        let values =
            get(source, &child_segments).map_err(|err| MetadatamodelError::Manipulation {
                message: err.to_string(),
            })?;
        let field_data: Option<&dyn Reflect> = values.first().copied();

        if !child.children.is_empty() {
            // nested group: branch per array element
            let is_sequence = field_data
                .map(|v| {
                    matches!(
                        v.reflect_ref().kind(),
                        dyn_rt_reflect::ReflectKind::Sequence
                    )
                })
                .unwrap_or(false);
            if is_sequence {
                let sequence = match field_data.unwrap().reflect_ref() {
                    ReflectRef::Sequence(seq) => seq,
                    _ => unreachable!(),
                };
                let mut collected: FlattenedTable = Vec::new();
                for i in 0..sequence.reflect_length() {
                    let mut indexes = linear_indexes.to_vec();
                    indexes.push(i);
                    let branch_rows =
                        recursive_convert(child, model, source, &indexes, current_rows.clone())?;
                    collected.extend(branch_rows);
                }
                current_rows = collected;
            } else {
                let mut indexes = linear_indexes.to_vec();
                indexes.push(0);
                current_rows = recursive_convert(child, model, source, &indexes, current_rows)?;
            }
            continue;
        }

        // leaf: append the cell to every current row
        let cell = cell_value(field_data);
        for row in &mut current_rows {
            row.push(cell.clone());
        }
    }

    Ok(current_rows)
}

/// Normalizes a found value into a cell: sequences stay whole, scalars are
/// wrapped in a single-element array, missing values become an empty array
/// (go's `getCellValueFromFieldData` / `DefaultEmptyColumn`).
fn cell_value(field_data: Option<&dyn Reflect>) -> Value {
    match field_data {
        None => Value::from(Vec::<Value>::new()),
        Some(value) => {
            if matches!(
                value.reflect_ref().kind(),
                dyn_rt_reflect::ReflectKind::Sequence
            ) {
                to_value(value)
            } else {
                Value::from(vec![to_value(value)])
            }
        }
    }
}

/// Converts a reflected value into a [`Value`] (for table cells).
pub fn to_value(value: &dyn Reflect) -> Value {
    if let Some(v) = value.reflect_as_any().downcast_ref::<Value>() {
        return v.clone();
    }
    match value.reflect_ref() {
        ReflectRef::Scalar(_) => {
            if let Some(b) = value.reflect_scalar_bool() {
                Value::from(b)
            } else if let Some(i) = value.reflect_scalar_i64() {
                Value::from(i)
            } else if let Some(f) = value.reflect_scalar_f64() {
                Value::from(f)
            } else if let Some(s) = value.reflect_scalar_string() {
                Value::from(s)
            } else {
                Value::Null
            }
        }
        ReflectRef::Sequence(seq) => Value::from(
            (0..seq.reflect_length())
                .filter_map(|i| seq.reflect_get(i))
                .map(to_value)
                .collect::<Vec<_>>(),
        ),
        ReflectRef::Map(map) => {
            let mut entries: Vec<(String, Value)> = Vec::new();
            let keys: Vec<&dyn Reflect> = map.reflect_iter_keys().collect();
            for key in keys {
                let key_string = to_value(key)
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                if let Some(v) = map.reflect_get(key) {
                    entries.push((key_string, to_value(v)));
                }
            }
            Value::object(entries)
        }
        ReflectRef::Struct(s) => {
            let mut entries: Vec<(String, Value)> = Vec::new();
            for info in s.reflect_field_infos() {
                if let Some(field) = s.reflect_field(info.name) {
                    entries.push((info.name.to_string(), to_value(field)));
                }
            }
            Value::object(entries)
        }
        ReflectRef::Enum(e) => {
            let variant = e.reflect_active_variant();
            if let Some(payload) = e.reflect_active_variant_payload() {
                return Value::object([
                    ("variant", Value::from(variant)),
                    ("payload", to_value(payload)),
                ]);
            }
            Value::from(variant)
        }
        ReflectRef::Indirection(i) => match i.reflect_inner_ref() {
            Some(inner) => to_value(inner),
            None => Value::Null,
        },
        ReflectRef::Unknown(_) => Value::Null,
    }
}

/// Rows are cloned when branching (go's copyTable): each branch gets its own
/// copy so appending cells in one branch does not affect the others. Rust's
/// `Vec<Value>` clone does exactly that via `current_rows.clone()` at the
/// branch call site.
pub fn copy_table(source: &FlattenedTable) -> FlattenedTable {
    source.clone()
}

/// Re-exported row type alias for callers building tables by hand.
pub type Row = FlattenedRow;
