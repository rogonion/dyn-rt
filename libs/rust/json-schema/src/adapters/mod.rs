//! Adapters for json-schema: the only place external crates beyond the
//! inward module deps (reflect, json-path, shared) may be imported. Text
//! deserialization via serde formats will live here behind cargo features.

use dyn_rt_json_path::{RegexPathTokenizer, parse_path};

#[cfg(feature = "deser")]
pub mod deser;
#[cfg(feature = "deser")]
pub use deser::*;

use crate::application::{SchemaAtPath, schema_at_path};
use crate::domain::Schema;
use crate::error::SchemaError;

/// Resolves a JSONPath string against `schema`. The path must not contain
/// the recursive descent operator `..` (go's `GetSchemaAtPath` rejects paths
/// with multiple recursive descent segments).
pub fn schema_at_path_str(path: &str, schema: &Schema) -> Result<SchemaAtPath, SchemaError> {
    let parsed = parse_path(&RegexPathTokenizer, path);
    if parsed.0.len() != 1 {
        return Err(SchemaError::SchemaPathError {
            message: "path contains multiple recursive descent segments".to_string(),
            path: path.to_string(),
        });
    }
    schema_at_path(&parsed.0[0], schema)
}
