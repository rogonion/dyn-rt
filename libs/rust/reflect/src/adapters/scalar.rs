//! Scalar adapters: the primitive types. A declarative macro is the
//! sanctioned deduplication here (ADR 0005). All listed types are `Clone`.

use std::any::{Any, type_name};

use crate::domain::{Reflect, ReflectKind, ReflectRef, ReflectRefMut};
use crate::error::ReflectError;

macro_rules! impl_reflect_scalar {
    ($($t:ty => $name:literal),* $(,)?) => {
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
            }
        )*
    };
}

impl_reflect_scalar!(
    bool => "bool",
    char => "char",
    i8 => "i8",
    i16 => "i16",
    i32 => "i32",
    i64 => "i64",
    i128 => "i128",
    isize => "isize",
    u8 => "u8",
    u16 => "u16",
    u32 => "u32",
    u64 => "u64",
    u128 => "u128",
    usize => "usize",
    f32 => "f32",
    f64 => "f64",
    String => "string",
);
