//! Closed error vocabulary for the json context.
//!
//! Adapters translate their native failures into these variants; nothing
//! else crosses the core boundary.

use std::fmt;

/// Errors produced by the json core (path parsing/resolution, object ops,
/// schema validation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonError {
    /// A JSONPath string could not be parsed.
    InvalidPath(String),
    /// A parsed path does not resolve against the document.
    PathNotFound(String),
    /// A path resolved to a value of an unexpected type.
    TypeMismatch {
        path: String,
        expected: &'static str,
        found: &'static str,
    },
    /// The document or one of its values is malformed for the requested
    /// operation (e.g. indexing into a non-array, setting a key on a scalar).
    InvalidValue(String),
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonError::InvalidPath(p) => write!(f, "invalid JSON path: {p}"),
            JsonError::PathNotFound(p) => write!(f, "path not found: {p}"),
            JsonError::TypeMismatch {
                path,
                expected,
                found,
            } => write!(
                f,
                "type mismatch at {path}: expected {expected}, found {found}"
            ),
            JsonError::InvalidValue(msg) => write!(f, "invalid value: {msg}"),
        }
    }
}

impl std::error::Error for JsonError {}

// Reason this suite stays inline (ADR 0004): Display strings are a local
// contract, not part of the cross-language vector format, so there is no
// shared fixture to drive them from.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_covers_all_variants() {
        let cases = [
            JsonError::InvalidPath("$..".into()),
            JsonError::PathNotFound("$.a.b".into()),
            JsonError::TypeMismatch {
                path: "$.a".into(),
                expected: "array",
                found: "string",
            },
            JsonError::InvalidValue("boom".into()),
        ];
        for case in cases {
            assert!(!case.to_string().is_empty());
        }
    }
}
