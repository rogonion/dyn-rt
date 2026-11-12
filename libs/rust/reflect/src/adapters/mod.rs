//! Reflect adapters: `impl Reflect` for `shared::Value` and std types —
//! one file per [`crate::domain::ReflectKind`] (ADR 0008), so the folder
//! listing shows which built-in types are covered. This is the only place in
//! the crate that imports `dyn-rt-shared`.

pub mod enum_shape;
pub mod indirection;
pub mod map;
pub mod scalar;
pub mod sequence;
pub mod unknown;
pub mod value;

use std::any::{Any, type_name};

use crate::domain::Reflect;
use crate::error::ReflectError;

pub(crate) fn unsupported(operation: &'static str, type_name: &str) -> ReflectError {
    ReflectError::UnsupportedOperation {
        operation,
        type_name: type_name.to_string(),
    }
}

/// Downcast a boxed reflected value into a concrete `Value`.
pub(crate) fn into_value(value: Box<dyn Reflect>) -> Result<dyn_rt_shared::Value, ReflectError> {
    let found = value.reflect_type_name().to_string();
    let any: Box<dyn Any> = value.reflect_into_any();
    any.downcast::<dyn_rt_shared::Value>()
        .map(|boxed| *boxed)
        .map_err(|_| ReflectError::ShapeMismatch {
            operation: "into_value",
            target: type_name::<dyn_rt_shared::Value>().to_string(),
            source: found,
        })
}

/// Downcast a boxed reflected value into a concrete `T`.
pub(crate) fn into_t<T: Reflect>(value: Box<dyn Reflect>) -> Result<T, ReflectError> {
    let found = value.reflect_type_name().to_string();
    let any: Box<dyn Any> = value.reflect_into_any();
    any.downcast::<T>()
        .map(|boxed| *boxed)
        .map_err(|_| ReflectError::ShapeMismatch {
            operation: "into_t",
            target: type_name::<T>().to_string(),
            source: found,
        })
}

/// Structural clone of a `T` out of a reflected value.
pub(crate) fn cloned_t<T: Reflect>(value: &dyn Reflect) -> T {
    let cloned: Box<dyn Any> = value.reflect_clone().reflect_into_any();
    *cloned
        .downcast::<T>()
        .expect("reflect_clone returns the same concrete type")
}

/// Extract a string key from an erased reflected key (`String`).
pub(crate) fn key_as_str(key: &dyn Reflect) -> Option<&str> {
    key.reflect_as_any()
        .downcast_ref::<String>()
        .map(|s| s.as_str())
}
