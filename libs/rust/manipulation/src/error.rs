//! The module's one closed error enum (ADR 0002). Adapters translate native
//! failures (path parsing, reflection, schema conversion) into these variants.

use std::fmt;

/// Everything that can go wrong while getting/setting/deleting at a JSONPath.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManipulationError {
    /// The path (or one of its segments) is structurally invalid — e.g. an
    /// empty segment list, an unsupported segment for the value's kind, a
    /// slice bound out of range (go's `ErrPathSegmentInvalidError`).
    PathSegmentInvalid { message: String },
    /// The value at a path segment is missing or not usable — e.g. a map key
    /// that does not exist, an index out of bounds, a nil source (go's
    /// `ErrValueAtPathSegmentInvalidError`).
    ValueAtPathSegmentInvalid { message: String },
    /// Converting the value to set (or a map key) into the destination type
    /// failed (wraps schema's conversion failure).
    ConversionFailed { message: String },
}

impl fmt::Display for ManipulationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManipulationError::PathSegmentInvalid { message } => {
                write!(f, "path segment invalid: {message}")
            }
            ManipulationError::ValueAtPathSegmentInvalid { message } => {
                write!(f, "value at path segment invalid: {message}")
            }
            ManipulationError::ConversionFailed { message } => {
                write!(f, "conversion failed: {message}")
            }
        }
    }
}

impl std::error::Error for ManipulationError {}
