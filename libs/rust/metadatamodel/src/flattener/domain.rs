//! Flattener subdomain domain (port of go-metadatamodel/flattener): the
//! 2D table types and the conversion tree.

use dyn_rt_shared::Value;

use crate::fieldcolumns::FieldColumnPosition;

/// A single row of a flattened table. Every cell is an array (slice) for
/// uniformity; empty cells are empty arrays (go's `FlattenedRow` /
/// `DefaultEmptyColumn`).
pub type FlattenedRow = Vec<Value>;

/// A 2D linear collection — the result of flattening an object (go's
/// `FlattenedTable`).
pub type FlattenedTable = Vec<FlattenedRow>;

/// A node of the conversion tree (go's `RecursiveIndexTree`): the column
/// position of a field, plus child nodes for a nested group.
#[derive(Debug, Clone)]
pub struct ConversionNode {
    /// The field/group this node reads. For a group node, `path_key` is the
    /// group's template; for a leaf, the field's (possibly separate-column)
    /// position.
    pub position: FieldColumnPosition,
    /// Child nodes when this node is a nested group.
    pub children: Vec<ConversionNode>,
}
