//! Unflattener subdomain application use case (port of
//! go-metadatamodel/unflattener): convert a 2D table back into a deeply
//! nested collection in a destination, grouping rows into instances by
//! signature (primary-key columns), and respecting field cardinality on
//! write (a `single` field's cell array is unwrapped; more than one value
//! errors unless [`SingleCellMode::TakeFirst`]).

use dyn_rt_json_manipulation::application::DynObject;
use dyn_rt_json_path::{AccessStrategy, PathSegment, PathSelector, RecursiveDescentSegments};
use dyn_rt_json_schema::Schema;
use dyn_rt_reflect::Reflect;
use dyn_rt_shared::Value;

use super::domain::{GroupCollection, GroupIndexNode, GroupIndexTree, Signature, SingleCellMode};
use crate::core::application::{
    cardinality_of, contains_nested_groups, data_path_model_aware, fields_of, is_group,
    parse_template, path_key_of, prop, props_of, read_order_of,
};
use crate::core::domain::{FIELDS_KEY, MetadataModel, props};
use crate::error::MetadatamodelError;
use crate::fieldcolumns::{
    ColumnFields, FieldColumnPosition, extract, group_column_indexes, max_flat_columns,
};
use crate::flattener::FlattenedTable;

/// Converts a 2D table into a nested collection written to `destination`
/// (go's `Unflattener`). The destination root is always a collection.
pub struct Unflattener<'a, 'd> {
    /// The metadata model describing the destination data.
    pub model: &'a MetadataModel,
    /// Extracted (and repositioned) column fields driving column order/skip.
    pub column_fields: ColumnFields,
    /// Where the reconstructed collection is written.
    pub destination: &'d mut dyn Reflect,
    /// Optional schema guiding set (creation/coercion).
    pub schema: Option<&'d Schema>,
    /// How to handle a `single`-cardinality field whose cell has >1 values.
    pub single_cell_mode: SingleCellMode,
    tree: GroupIndexTree,
    index: GroupCollection,
}

impl<'a, 'd> Unflattener<'a, 'd> {
    /// Creates an unflattener, extracting the default column fields from the
    /// model (go's `NewUnflattener` + default extraction in `Unflatten`).
    pub fn new(
        model: &'a MetadataModel,
        destination: &'d mut dyn Reflect,
        schema: Option<&'d Schema>,
    ) -> Result<Self, MetadatamodelError> {
        let mut column_fields = extract(model, &Default::default())?;
        column_fields.reposition();
        column_fields.update_index_in_repositioned();
        Self::with_column_fields(model, column_fields, destination, schema)
    }

    /// Creates an unflattener with caller-provided column fields.
    pub fn with_column_fields(
        model: &'a MetadataModel,
        column_fields: ColumnFields,
        destination: &'d mut dyn Reflect,
        schema: Option<&'d Schema>,
    ) -> Result<Self, MetadatamodelError> {
        let tree = build_group_index_tree(model, "$", &column_fields)?;
        Ok(Unflattener {
            model,
            column_fields,
            destination,
            schema,
            single_cell_mode: SingleCellMode::default(),
            tree,
            index: GroupCollection::default(),
        })
    }

    /// Processes every row of `table` into the destination (go's `Unflatten`).
    pub fn unflatten(&mut self, table: &FlattenedTable) -> Result<(), MetadatamodelError> {
        for row in table {
            recursive_convert(
                &self.tree,
                self.model,
                &self.column_fields,
                self.destination,
                self.schema,
                self.single_cell_mode,
                &mut self.index,
                &[],
                row,
            )?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// group index tree (go's recursiveInitGroupIndexTree)
// ---------------------------------------------------------------------------

fn build_group_index_tree(
    group: &MetadataModel,
    group_path_key: &str,
    column_fields: &ColumnFields,
) -> Result<GroupIndexTree, MetadatamodelError> {
    let column_indexes = group_column_indexes(column_fields, group)?;
    let mut tree = GroupIndexTree {
        path_key: group_path_key.to_string(),
        suffix: String::new(),
        column_indexes,
        children: Vec::new(),
    };

    let order = read_order_of(group)?;
    let fields_obj =
        fields_of(group)?
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
            if as_single {
                continue; // processed as a plain field (its columns are already indexed)
            }
            let max_columns = max_flat_columns(props)?;
            if !contains_nested_groups(props)? && max_columns > 0 {
                // group-as-separate-columns: no child node (go skips it)
                continue;
            }
            match build_group_index_tree(props, &path_key, column_fields) {
                Ok(mut child) => {
                    child.suffix = key.clone();
                    tree.children.push(child);
                }
                Err(err) => {
                    if !err.to_string().contains("no group fields") {
                        return Err(err);
                    }
                }
            }
        }
    }
    Ok(tree)
}

// ---------------------------------------------------------------------------
// recursive convert (go's recursiveConvert)
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn recursive_convert(
    tree: &GroupIndexTree,
    model: &MetadataModel,
    column_fields: &ColumnFields,
    destination: &mut dyn Reflect,
    schema: Option<&Schema>,
    single_cell_mode: SingleCellMode,
    parent_collection: &mut GroupCollection,
    linear_indexes: &[usize],
    row: &[Value],
) -> Result<(), MetadatamodelError> {
    // 1. signature from primary key columns (else all columns)
    let signature_columns = if !tree.column_indexes.primary.is_empty() {
        &tree.column_indexes.primary
    } else {
        &tree.column_indexes.all
    };
    let signature = Signature::default().generate_signature(row, signature_columns);

    // 2. get/create the instance
    let (instance_index, node) =
        parent_collection.get_or_create_instance(&signature, &tree.path_key);
    let mut current_path_indexes = linear_indexes.to_vec();
    current_path_indexes.push(instance_index);

    // 3. write this group's field columns
    for col_index in &tree.column_indexes.all {
        let Some(cell) = row.get(*col_index) else {
            continue;
        };
        if cell.is_null() {
            continue;
        }
        let Some(original) = column_fields.current_read_order().get(*col_index).copied() else {
            continue;
        };
        let Some(position) = column_fields.read_order.get(original) else {
            continue;
        };
        let Some(col_field) = column_fields.get_by_index_in_original_read_order(original) else {
            continue;
        };

        let target = leaf_target_path(position, model, &current_path_indexes)?;
        let value_to_write = value_for_cell(cell, &col_field.props, single_cell_mode)?;
        // `set` converts into the existing value, which appends to sequences;
        // a whole-field (key-terminal) write must replace, so zero the target
        // first (delete on a struct field zeroes it; indexed separate-column
        // slots already replace in place and are left alone).
        let key_terminal = target
            .0
            .last()
            .and_then(|group| group.0.last())
            .is_some_and(|segment| matches!(segment.selector, PathSelector::Key(_)));
        if key_terminal {
            let _ = DynObject::new(&mut *destination).delete(&target);
        }
        DynObject::new(&mut *destination)
            .with_schema_opt(schema)
            .set(&target, &value_to_write)
            .map_err(|err| MetadatamodelError::Manipulation {
                message: format!("write field failed: {err}"),
            })?;
    }

    // 4. recurse into nested groups
    for child in &tree.children {
        let child_collection = node.get_or_create_group(&child.suffix);
        recursive_convert(
            child,
            model,
            column_fields,
            destination,
            schema,
            single_cell_mode,
            child_collection,
            &current_path_indexes,
            row,
        )?;
    }

    Ok(())
}

/// The target path a cell is written to: the destination root is a
/// collection, so the first path index addresses the root entry; the rest
/// address the nested groups. Separate-column leaves additionally index the
/// field's own array with their header index.
fn leaf_target_path(
    position: &FieldColumnPosition,
    model: &MetadataModel,
    path_indexes: &[usize],
) -> Result<RecursiveDescentSegments, MetadatamodelError> {
    let Some((root_index, rest)) = path_indexes.split_first() else {
        return Err(MetadatamodelError::Unflatten {
            message: "no root instance index".to_string(),
        });
    };

    let mut indexes = rest.to_vec();
    let index_last = if position.group_in_separate_columns {
        indexes.push(position.group_header_index);
        true
    } else if position.field_in_separate_columns {
        indexes.push(position.field_header_index);
        true
    } else {
        false
    };

    let path = if position.group_in_separate_columns {
        let parent = data_path_model_aware(&position.group_parent_path_key, model, &indexes, true)?;
        format!("{parent}.{}", position.field_key_suffix)
    } else {
        data_path_model_aware(&position.path_key, model, &indexes, index_last)?
    };
    let mut segments = parse_template(&path)?;
    if let Some(first_group) = segments.0.first_mut() {
        first_group.0.insert(
            1,
            PathSegment {
                access_strategy: AccessStrategy::Positional,
                selector: PathSelector::Index(*root_index as isize),
            },
        );
    }
    Ok(segments)
}

/// Applies cardinality to a cell before writing: a `single` field's cell
/// array is unwrapped (erroring on >1 values unless TakeFirst); an `array`
/// field's cell is written as-is. Separate-column slots always take the
/// cell's first element.
fn value_for_cell(
    cell: &Value,
    field_props: &Value,
    single_cell_mode: SingleCellMode,
) -> Result<Value, MetadatamodelError> {
    let cardinality = cardinality_of(field_props);
    if cardinality == crate::core::domain::Cardinality::Single {
        match cell {
            Value::Array(items) => match items.len() {
                0 => Ok(Value::Null),
                1 => Ok(items[0].clone()),
                _ => match single_cell_mode {
                    SingleCellMode::Error => Err(MetadatamodelError::Unflatten {
                        message: format!(
                            "single-cardinality field cell has {} values",
                            items.len()
                        ),
                    }),
                    SingleCellMode::TakeFirst => Ok(items[0].clone()),
                },
            },
            other => Ok(other.clone()),
        }
    } else {
        Ok(cell.clone())
    }
}

/// Re-export so callers can name the node type.
pub type Node = GroupIndexNode;
