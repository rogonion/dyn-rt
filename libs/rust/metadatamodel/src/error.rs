//! One closed error enum for the metadatamodel module (ADR 0002).

use std::fmt;

/// Errors raised by the metadatamodel submodules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetadatamodelError {
    /// The metadata model (or a field/group within it) is not shaped as
    /// expected (missing `fields` array, property of the wrong type, ...).
    InvalidModel { message: String },
    /// A required property is absent.
    MissingProperty { key: &'static str },
    /// A value's shape contradicts the field/group's declared cardinality
    /// (`array` declared but value is not a sequence, `single` declared but
    /// value is a sequence). Strict: no dynamic exceptions.
    CardinalityViolation { message: String },
    /// A `group:path_key` template is not a valid/usable JSONPath template.
    PathTemplateInvalid { message: String },
    /// database `get_column_fields` failed (missing table criterion,
    /// duplicate `db:column_name`, field without `db:column_name`, ...).
    GetColumnFields { message: String },
    /// database field-value get/set/delete failed (unknown column name,
    /// unusable path, ...).
    FieldValue { message: String },
    /// An underlying json-manipulation operation failed.
    Manipulation { message: String },
    /// filter subdomain: query-condition evaluation failed (invalid query
    /// shape, unknown condition type, unsupported filter condition, ...).
    Filter { message: String },
}

impl fmt::Display for MetadatamodelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetadatamodelError::InvalidModel { message } => {
                write!(f, "invalid metadata model: {message}")
            }
            MetadatamodelError::MissingProperty { key } => {
                write!(f, "missing property {key:?}")
            }
            MetadatamodelError::CardinalityViolation { message } => {
                write!(f, "cardinality violation: {message}")
            }
            MetadatamodelError::PathTemplateInvalid { message } => {
                write!(f, "path template invalid: {message}")
            }
            MetadatamodelError::GetColumnFields { message } => {
                write!(f, "get column fields: {message}")
            }
            MetadatamodelError::FieldValue { message } => {
                write!(f, "field value: {message}")
            }
            MetadatamodelError::Manipulation { message } => {
                write!(f, "manipulation failed: {message}")
            }
            MetadatamodelError::Filter { message } => {
                write!(f, "filter failed: {message}")
            }
        }
    }
}

impl std::error::Error for MetadatamodelError {}
