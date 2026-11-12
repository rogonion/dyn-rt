//! Helpers shared by the reflect test suites (each suite file is its own test
//! crate, so this module is included per crate).
#![allow(dead_code, unused_macros, unused_imports)]

use dyn_rt_reflect::{Reflect, ReflectError};
use dyn_rt_shared::Value;

/// Build an object value from entries (key order kept, duplicate keys replace).
pub fn obj(entries: Vec<(&str, Value)>) -> Value {
    Value::object(entries)
}

/// Build an array value.
pub fn arr(items: Vec<Value>) -> Value {
    Value::from(items)
}

/// Downcast a reflected value back to the domain `Value` (panics if not one).
pub fn value_of(reflected: &dyn Reflect) -> &Value {
    reflected
        .reflect_as_any()
        .downcast_ref::<Value>()
        .expect("reflected value is a Value")
}

/// The variant name of a ReflectError, for expected-vs-got reporting.
pub fn error_kind(err: &ReflectError) -> &'static str {
    match err {
        ReflectError::UnsupportedOperation { .. } => "UnsupportedOperation",
        ReflectError::KeyNotFound { .. } => "KeyNotFound",
        ReflectError::IndexOutOfBounds { .. } => "IndexOutOfBounds",
        ReflectError::ShapeMismatch { .. } => "ShapeMismatch",
        ReflectError::InnerBorrowFailed { .. } => "InnerBorrowFailed",
        ReflectError::InnerNotProvided { .. } => "InnerNotProvided",
        ReflectError::UnknownVariant { .. } => "UnknownVariant",
        _ => "Other",
    }
}
