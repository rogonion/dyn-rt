//! Closed error vocabulary for the reflect module.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReflectError {
    /// The operation is not supported by this value's shape.
    UnsupportedOperation {
        operation: &'static str,
        type_name: String,
    },
    /// A value of the wrong concrete type was supplied where another was
    /// expected (e.g. a non-`Value` written into a `Value` slot).
    TypeMismatch {
        expected: &'static str,
        found: String,
    },
    /// A sequence index was outside `0..len`.
    IndexOutOfBounds { index: usize, len: usize },
}

impl fmt::Display for ReflectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReflectError::UnsupportedOperation {
                operation,
                type_name,
            } => {
                write!(f, "unsupported operation '{operation}' on {type_name}")
            }
            ReflectError::TypeMismatch { expected, found } => {
                write!(f, "type mismatch: expected {expected}, found {found}")
            }
            ReflectError::IndexOutOfBounds { index, len } => {
                write!(f, "index {index} out of bounds (len {len})")
            }
        }
    }
}

impl std::error::Error for ReflectError {}
