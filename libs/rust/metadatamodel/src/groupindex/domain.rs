//! Group-index domain (port of go-metadatamodel/unflattener's index.go,
//! extracted into its own subdomain): a node of instances, the collection
//! of a group's instances keyed by signature, and the signature generator.

use dyn_rt_shared::Value;

/// A specific instance of a group element (e.g. "Employee #1") (go's
/// `GroupIndexNode`).
#[derive(Debug, Default)]
pub struct GroupIndexNode {
    pub path_key: String,
    /// The resolved index of this node in its parent's list.
    pub my_index: usize,
    /// Nested group collections, keyed by group suffix.
    pub groups: Vec<(String, GroupCollection)>,
}

impl GroupIndexNode {
    /// Retrieves or creates the child collection for a nested group suffix.
    pub fn get_or_create_group(&mut self, suffix: &str) -> &mut GroupCollection {
        if let Some(position) = self.groups.iter().position(|(s, _)| s == suffix) {
            &mut self.groups[position].1
        } else {
            self.groups
                .push((suffix.to_string(), GroupCollection::default()));
            &mut self.groups.last_mut().unwrap().1
        }
    }
}

/// A "list" of child nodes of one group (go's `GroupCollection`).
#[derive(Debug, Default)]
pub struct GroupCollection {
    /// Counter for the next available index in this list.
    pub next_index: usize,
    /// Instances keyed by signature (insertion-ordered).
    pub instances: Vec<(String, GroupIndexNode)>,
}

impl GroupCollection {
    /// Retrieves or creates the instance for `signature`, returning its node
    /// and its index in this list (go's `GetOrCreateInstance`).
    pub fn get_or_create_instance(
        &mut self,
        signature: &str,
        path_key: &str,
    ) -> (usize, &mut GroupIndexNode) {
        if let Some(position) = self.instances.iter().position(|(s, _)| s == signature) {
            let index = self.instances[position].1.my_index;
            return (index, &mut self.instances[position].1);
        }
        let index = self.next_index;
        self.next_index += 1;
        self.instances.push((
            signature.to_string(),
            GroupIndexNode {
                path_key: path_key.to_string(),
                my_index: index,
                groups: Vec::new(),
            },
        ));
        let last = self.instances.len() - 1;
        (index, &mut self.instances[last].1)
    }
}

/// Generates unique row keys from primary-key columns (go's `Signature`).
#[derive(Debug, Clone)]
pub struct Signature {
    pub join_symbol: char,
}

impl Default for Signature {
    fn default() -> Self {
        Signature { join_symbol: '|' }
    }
}

impl Signature {
    /// A deterministic key for `row` over the given column indexes.
    pub fn generate_signature(&self, row: &[Value], columns: &[usize]) -> String {
        if columns.is_empty() {
            return String::new();
        }
        let mut parts: Vec<String> = Vec::new();
        for col in columns {
            let Some(value) = row.get(*col) else {
                parts.push("nil".to_string());
                continue;
            };
            parts.push(render_signature_value(value));
        }
        parts.join(&self.join_symbol.to_string())
    }
}

/// Renders one cell value for a signature: strings as-is, numbers/bools
/// compactly, null/empty as `nil`.
fn render_signature_value(value: &Value) -> String {
    match value {
        Value::Null => "nil".to_string(),
        Value::Bool(b) => {
            if *b {
                "t".to_string()
            } else {
                "f".to_string()
            }
        }
        Value::Number(n) => n.as_f64().to_string(),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(render_signature_value)
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => format!("{value:?}"),
    }
}
