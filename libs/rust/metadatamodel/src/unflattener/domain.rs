//! Unflattener subdomain domain (port of go-metadatamodel/unflattener): the
//! group index tree (the model->tree bridge over fieldcolumns) and the
//! single-cell cardinality mode. The reusable instance/signature machinery
//! itself lives in the `groupindex` subdomain and is re-exported here.

use crate::fieldcolumns::GroupColumnIndexes;

pub use crate::groupindex::{GroupCollection, GroupIndexNode, Signature};

/// What to do when a `single`-cardinality field's cell holds more than one
/// value (the unflattener respects cardinality: a single value is unwrapped
/// from the cell array).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SingleCellMode {
    /// Error out (strict cardinality).
    #[default]
    Error,
    /// Take the first value and discard the rest.
    TakeFirst,
}

/// A node of the group index tree (go's `RecursiveGroupIndexTree`): which
/// columns belong to a group plus its nested group nodes. The runtime
/// instance structures ([`GroupIndexNode`], [`GroupCollection`],
/// [`Signature`]) live in the `groupindex` subdomain.
#[derive(Debug, Clone)]
pub struct GroupIndexTree {
    /// The group's schema path key.
    pub path_key: String,
    /// The field-name suffix of this group within its parent (empty at root).
    pub suffix: String,
    /// Which columns (unskipped current read-order indexes) belong to this group.
    pub column_indexes: GroupColumnIndexes,
    /// Nested group nodes.
    pub children: Vec<GroupIndexTree>,
}
