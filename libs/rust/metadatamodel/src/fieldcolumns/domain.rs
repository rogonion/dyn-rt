//! fieldcolumns domain: the metadata model's fields as columns of a flat
//! 2D table (port of go-metadatamodel/fieldcolumns core.go).

use dyn_rt_json_schema::Schema;
use dyn_rt_shared::Value;

/// Where a column sits: the field's `group:path_key`, optionally expanded
/// into separate columns (one per value index) for fields/groups with
/// `view:values_in_separate_columns`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FieldColumnPosition {
    /// The field's `group:path_key` template.
    pub path_key: String,
    pub field_in_separate_columns: bool,
    pub field_header_index: usize,
    pub group_in_separate_columns: bool,
    pub group_header_index: usize,
    pub group_parent_path_key: String,
    pub field_key_suffix: String,
    /// `view:position_before`: insert before the destination instead of after.
    pub position_before: bool,
    /// Original read-order index of the source column (set when the
    /// position is recorded as a reposition instruction).
    pub source_index: usize,
}

impl FieldColumnPosition {
    /// The key identifying this column in [`ColumnFields::fields`]: for
    /// group-separate-columns `parent[i].suffix`, for field-separate-columns
    /// `path_key[i]`, else the plain `path_key`.
    pub fn column_key(&self) -> String {
        if self.group_in_separate_columns {
            format!(
                "{}[{}].{}",
                self.group_parent_path_key, self.group_header_index, self.field_key_suffix
            )
        } else if self.field_in_separate_columns {
            format!("{}[{}]", self.path_key, self.field_header_index)
        } else {
            self.path_key.clone()
        }
    }
}

/// One extracted column: the field's props (deep-copied and merged with any
/// matched parent props) plus bookkeeping.
#[derive(Debug, Clone)]
pub struct ColumnField {
    /// The field/group props object (owned copy — the model is not mutated).
    pub props: Value,
    /// The field's schema, when an extraction schema was provided.
    pub schema: Option<Schema>,
    /// Index in the original read order (extraction order).
    pub index_in_read_order: usize,
    /// Index after repositioning (see [`ColumnFields::reposition`]).
    pub index_in_repositioned: usize,
    /// Set by [`ColumnFields::skip`].
    pub skip: bool,
}

/// The extraction result: fields as columns in a flat 2D table.
#[derive(Debug, Clone, Default)]
pub struct ColumnFields {
    /// Column key → field, in insertion order.
    pub fields: Vec<(String, ColumnField)>,
    /// Read order of the columns (positions, incl. separate-column expansion).
    pub read_order: Vec<FieldColumnPosition>,
    /// Current order as indexes into `read_order`; mutated by reposition.
    pub current: Vec<usize>,
    /// Reposition instructions (from `view:column_position` props).
    pub reposition: Vec<FieldColumnPosition>,
}

impl ColumnFields {
    /// Looks a column up by its column key.
    pub fn get_by_key(&self, column_key: &str) -> Option<&ColumnField> {
        self.fields
            .iter()
            .find(|(key, _)| key == column_key)
            .map(|(_, field)| field)
    }

    /// Looks a column up by index into the current (possibly repositioned)
    /// read order.
    pub fn get_by_index_in_current_read_order(&self, index: usize) -> Option<&ColumnField> {
        let original = *self.current.get(index)?;
        self.get_by_index_in_original_read_order(original)
    }

    /// Looks a column up by index into the original read order.
    pub fn get_by_index_in_original_read_order(&self, index: usize) -> Option<&ColumnField> {
        let position = self.read_order.get(index)?;
        self.get_by_key(&position.column_key())
    }

    /// Current read order with skipped fields removed.
    pub fn current_read_order(&self) -> Vec<usize> {
        self.current
            .iter()
            .copied()
            .filter(|original| {
                self.get_by_index_in_original_read_order(*original)
                    .is_none_or(|field| !field.skip)
            })
            .collect()
    }

    /// Applies the recorded reposition instructions to `current` (go's
    /// Reposition): each instruction moves `source_index` (an original
    /// read-order index) before/after the destination column.
    pub fn reposition(&mut self) {
        let total = self.current.len();
        for instruction in self.reposition.clone() {
            let Some(destination_original) = self
                .read_order
                .iter()
                .position(|p| p.column_key() == instruction.column_key())
            else {
                continue;
            };
            let Some(destination_current) = self
                .current
                .iter()
                .position(|original| *original == destination_original)
            else {
                continue;
            };
            let Some(source_current) = self
                .current
                .iter()
                .position(|original| *original == instruction.source_index)
            else {
                continue;
            };
            let destination_current =
                if instruction.position_before || destination_current >= total - 1 {
                    destination_current
                } else {
                    destination_current + 1
                };
            if destination_current != source_current {
                let source = self.current.remove(source_current);
                self.current
                    .insert(destination_current.min(self.current.len()), source);
            }
        }
    }

    /// Updates each column's `index_in_repositioned` from `current`.
    pub fn update_index_in_repositioned(&mut self) {
        for (read_index, original) in self.current.iter().enumerate() {
            if let Some((_, field)) = self
                .fields
                .iter_mut()
                .find(|(_, f)| f.index_in_read_order == *original)
            {
                field.index_in_repositioned = read_index;
            }
        }
    }
}
