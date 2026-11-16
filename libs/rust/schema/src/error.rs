//! The closed error enum for the schema module (ADR 0002: one closed
//! error enum per module core; adapters translate native failures into it).

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    /// Data did not validate against the schema.
    ValidationFailed { message: String, path: String },
    /// A schema path could not be resolved against the schema.
    SchemaPathError { message: String, path: String },
    /// The schema itself is not usable in this position.
    UnsupportedSchema { message: String },
    /// Conversion of data into the schema-dictated shape failed.
    ConversionFailed { message: String, path: String },
    /// Deserialization from an external format failed (adapters only).
    DeserializationFailed { message: String },
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchemaError::ValidationFailed { message, path } => {
                write!(f, "validation failed at '{path}': {message}")
            }
            SchemaError::SchemaPathError { message, path } => {
                write!(f, "schema path error at '{path}': {message}")
            }
            SchemaError::UnsupportedSchema { message } => {
                write!(f, "unsupported schema: {message}")
            }
            SchemaError::ConversionFailed { message, path } => {
                write!(f, "conversion failed at '{path}': {message}")
            }
            SchemaError::DeserializationFailed { message } => {
                write!(f, "deserialization failed: {message}")
            }
        }
    }
}

impl std::error::Error for SchemaError {}
