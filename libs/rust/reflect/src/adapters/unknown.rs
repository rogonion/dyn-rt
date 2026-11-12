//! Unknown-kind adapters: types that implement only the base `Reflect` trait
//! and route to `Unknown` — no shape trait forced on them (ADR 0008).

use std::any::{Any, type_name};

use crate::domain::{Reflect, ReflectKind, ReflectRef, ReflectRefMut};
use crate::error::ReflectError;

/// Base-`Reflect`-only impl for an uncovered type: routes to `Unknown`,
/// clones via `Clone`, and rejects `reflect_apply` from other types.
macro_rules! impl_reflect_unknown {
    ($($t:ty => $name:literal),* $(,)?) => {
        $(
            impl Reflect for $t {
                fn reflect_type_name(&self) -> &'static str {
                    $name
                }

                fn reflect_ref(&self) -> ReflectRef<'_> {
                    ReflectRef::Unknown(self)
                }

                fn reflect_ref_mut(&mut self) -> ReflectRefMut<'_> {
                    ReflectRefMut::Unknown(self)
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
                    ReflectKind::Unknown
                }
            }
        )*
    };
}

// Example of an uncovered-but-representable type:
impl_reflect_unknown!(std::time::Duration => "Duration");
