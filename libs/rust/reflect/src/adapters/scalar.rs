//! Scalar adapters: the primitive types. A declarative macro is the
//! sanctioned deduplication here (ADR 0005). All listed types are `Clone`.

use std::any::{Any, type_name};

use crate::domain::{Reflect, ReflectKind, ReflectNew, ReflectRef, ReflectRefMut};
use crate::error::ReflectError;

macro_rules! impl_reflect_scalar {
    ($($t:ty => $name:literal, i64: $i64:expr, f64: $f64:expr, bool: $bool:expr, string: $string:expr),* $(,)?) => {
        $(
            impl Reflect for $t {
                fn reflect_type_name(&self) -> &'static str {
                    $name
                }

                fn reflect_ref(&self) -> ReflectRef<'_> {
                    ReflectRef::Scalar(self)
                }

                fn reflect_ref_mut(&mut self) -> ReflectRefMut<'_> {
                    ReflectRefMut::Scalar(self)
                }

                fn reflect_clone(&self) -> Box<dyn Reflect> {
                    Box::new(self.clone())
                }

                fn reflect_zero(&self) -> Option<Box<dyn Reflect>> {
                    Some(Box::new(<$t as Default>::default()))
                }

                fn reflect_apply(&mut self, value: &dyn Reflect) -> Result<(), ReflectError> {
                    let source = value.reflect_as_any().downcast_ref::<$t>().ok_or_else(|| {
                        ReflectError::ShapeMismatch {
                            operation: "reflect_apply",
                            target: type_name::<$t>().to_string(),
                            source: value.reflect_type_name().to_string(),
                        }
                    })?;
                    *self = source.clone();
                    Ok(())
                }

                fn reflect_as_any(&self) -> &dyn Any {
                    self
                }

                fn reflect_as_any_mut(&mut self) -> &mut dyn Any {
                    self
                }

                fn reflect_into_any(self: Box<Self>) -> Box<dyn Any> {
                    self
                }

                fn reflect_kind() -> ReflectKind {
                    ReflectKind::Scalar
                }

                fn reflect_scalar_i64(&self) -> Option<i64> {
                    let f: fn(&$t) -> Option<i64> = $i64;
                    f(self)
                }

                fn reflect_scalar_f64(&self) -> Option<f64> {
                    let f: fn(&$t) -> Option<f64> = $f64;
                    f(self)
                }

                fn reflect_scalar_bool(&self) -> Option<bool> {
                    let f: fn(&$t) -> Option<bool> = $bool;
                    f(self)
                }

                fn reflect_scalar_string(&self) -> Option<String> {
                    let f: fn(&$t) -> Option<String> = $string;
                    f(self)
                }
            }
        )*
    };
}

impl_reflect_scalar!(
    bool => "bool", i64: |_| None, f64: |_| None, bool: |b| Some(*b), string: |b| Some(b.to_string()),
    char => "char", i64: |c| Some(*c as i64), f64: |c| Some(*c as u32 as f64), bool: |_| None, string: |c| Some(c.to_string()),
    i8 => "i8", i64: |v| Some(*v as i64), f64: |v| Some(*v as f64), bool: |_| None, string: |v| Some(v.to_string()),
    i16 => "i16", i64: |v| Some(*v as i64), f64: |v| Some(*v as f64), bool: |_| None, string: |v| Some(v.to_string()),
    i32 => "i32", i64: |v| Some(*v as i64), f64: |v| Some(*v as f64), bool: |_| None, string: |v| Some(v.to_string()),
    i64 => "i64", i64: |v| Some(*v), f64: |v| Some(*v as f64), bool: |_| None, string: |v| Some(v.to_string()),
    i128 => "i128", i64: |v| i64::try_from(*v).ok(), f64: |v| Some(*v as f64), bool: |_| None, string: |v| Some(v.to_string()),
    isize => "isize", i64: |v| Some(*v as i64), f64: |v| Some(*v as f64), bool: |_| None, string: |v| Some(v.to_string()),
    u8 => "u8", i64: |v| Some(*v as i64), f64: |v| Some(*v as f64), bool: |_| None, string: |v| Some(v.to_string()),
    u16 => "u16", i64: |v| Some(*v as i64), f64: |v| Some(*v as f64), bool: |_| None, string: |v| Some(v.to_string()),
    u32 => "u32", i64: |v| Some(*v as i64), f64: |v| Some(*v as f64), bool: |_| None, string: |v| Some(v.to_string()),
    u64 => "u64", i64: |v| i64::try_from(*v).ok(), f64: |v| Some(*v as f64), bool: |_| None, string: |v| Some(v.to_string()),
    u128 => "u128", i64: |v| i64::try_from(*v).ok(), f64: |v| Some(*v as f64), bool: |_| None, string: |v| Some(v.to_string()),
    usize => "usize", i64: |v| Some(*v as i64), f64: |v| Some(*v as f64), bool: |_| None, string: |v| Some(v.to_string()),
    f32 => "f32", i64: |_| None, f64: |v| Some(*v as f64), bool: |_| None, string: |v| Some(v.to_string()),
    f64 => "f64", i64: |_| None, f64: |v| Some(*v), bool: |_| None, string: |v| Some(v.to_string()),
    String => "string", i64: |_| None, f64: |_| None, bool: |_| None, string: |s| Some(s.clone()),
    // unit: the scalar with no payload; renders like Value::Null ("null")
    () => "unit", i64: |_| None, f64: |_| None, bool: |_| None, string: |_| Some("null".to_string()),
);

/// Scalars are constructible via their `Default` (zero value).
macro_rules! impl_reflect_new_scalar {
    ($($t:ty),* $(,)?) => {
        $(
            impl ReflectNew for $t {
                fn reflect_new() -> Box<dyn Reflect> {
                    Box::new(<$t>::default())
                }
            }
        )*
    };
}

impl_reflect_new_scalar!(
    bool,
    char,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    f32,
    f64,
    String,
    (),
);
