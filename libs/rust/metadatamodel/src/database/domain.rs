//! Database subdomain domain: the columns of one database table collection
//! (port of go-metadatamodel/database's `ColumnFields`).

use dyn_rt_shared::Value;

/// Columns/fields of a particular `db:table_collection_name` at a specific
/// `db:join_depth`, extracted from a metadata model by
/// [`crate::database::application::get_column_fields`].
///
/// The key of each entry is the field's `db:column_name`.
#[derive(Debug, Clone, Default)]
pub struct ColumnFields {
    /// Read order of the column names.
    pub read_order: Vec<String>,
    /// The field/group props of each column, keyed by `db:column_name`.
    pub fields: Vec<(String, Value)>,
}

impl ColumnFields {
    pub fn new() -> Self {
        Self::default()
    }

    /// The field/group props of the column with this `db:column_name`.
    pub fn get(&self, column_name: &str) -> Option<&Value> {
        self.fields
            .iter()
            .find(|(key, _)| key == column_name)
            .map(|(_, props)| props)
    }
}
