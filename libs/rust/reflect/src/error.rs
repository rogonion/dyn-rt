//! Closed error vocabulary for the reflect module (ported from
//! dyn-rt-rs-old-3). `#[non_exhaustive]`: downstream consumers match with a
//! catch-all arm so new variants are not breaking changes.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReflectError {
    /// The operation does not exist for this type or shape.
    UnsupportedOperation {
        operation: &'static str,
        type_name: String,
    },
    /// The key (or field name) does not exist in the keyed value.
    KeyNotFound { key: String, type_name: String },
    /// The index is beyond the sequence bounds.
    IndexOutOfBounds {
        index: usize,
        length: usize,
        type_name: String,
    },
    /// `reflect_apply` was called between values whose types do not match.
    ShapeMismatch {
        operation: &'static str,
        target: String,
        source: String,
    },
    /// The inner value of an indirection could not be borrowed (e.g. a
    /// `RefCell` borrow conflict, a poisoned mutex).
    InnerBorrowFailed { type_name: String },
    /// The indirection implementation did not provide an inner value to the
    /// read callback. An implementation bug: callbacks must fire exactly once.
    InnerNotProvided { type_name: String },
    /// The enum does not have a variant with this name.
    UnknownVariant { variant: String, type_name: String },
}

impl fmt::Display for ReflectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReflectError::UnsupportedOperation {
                operation,
                type_name,
            } => {
                write!(
                    f,
                    "unsupported operation '{operation}' for type '{type_name}'"
                )
            }
            ReflectError::KeyNotFound { key, type_name } => {
                write!(f, "key '{key}' not found in type '{type_name}'")
            }
            ReflectError::IndexOutOfBounds {
                index,
                length,
                type_name,
            } => write!(
                f,
                "index {index} out of bounds for length {length} in type '{type_name}'"
            ),
            ReflectError::ShapeMismatch {
                operation,
                target,
                source,
            } => write!(
                f,
                "shape mismatch in '{operation}': target '{target}', source '{source}'"
            ),
            ReflectError::InnerBorrowFailed { type_name } => {
                write!(f, "failed to borrow inner value of type '{type_name}'")
            }
            ReflectError::InnerNotProvided { type_name } => {
                write!(
                    f,
                    "indirection of type '{type_name}' did not provide an inner value"
                )
            }
            ReflectError::UnknownVariant { variant, type_name } => {
                write!(f, "unknown variant '{variant}' for type '{type_name}'")
            }
        }
    }
}

impl std::error::Error for ReflectError {}
