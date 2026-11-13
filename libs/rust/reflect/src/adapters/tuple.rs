//! Tuple adapters: `impl Reflect` for tuples of arity 1..=16. A tuple routes
//! to the struct shape with positional field names ("0", "1", ...) — the same
//! convention serde uses for tuple structs — because tuple elements may be
//! heterogeneous, so the homogeneous sequence shape does not fit. (The
//! 0-arity tuple is `()`, covered as a scalar in `scalar.rs`.)
//!
//! Enum tuple variants are already covered by the enum shape derive
//! (`KeyPress(char)` style payloads), so no enum-side work is needed here.
//!
//! A declarative macro is the sanctioned deduplication across arities
//! (ADR 0005). Element bounds are `Reflect + Clone + Default` so that clone,
//! apply and zero/new all work element-wise.

use std::any::{Any, type_name};
use std::sync::OnceLock;

use crate::domain::{
    FieldInfo, Reflect, ReflectKind, ReflectNew, ReflectRef, ReflectRefMut, ReflectStruct,
    type_info,
};
use crate::error::ReflectError;

macro_rules! impl_reflect_tuple {
    ($(($($idx:tt => $t:ident),+ $(,)?)),+ $(,)?) => {
        $(
            impl<$($t: Reflect + Clone + Default),+> Reflect for ($($t,)+) {
                fn reflect_type_name(&self) -> &'static str {
                    type_name::<Self>()
                }

                fn reflect_ref(&self) -> ReflectRef<'_> {
                    ReflectRef::Struct(self)
                }

                fn reflect_ref_mut(&mut self) -> ReflectRefMut<'_> {
                    ReflectRefMut::Struct(self)
                }

                fn reflect_clone(&self) -> Box<dyn Reflect> {
                    Box::new(self.clone())
                }

                fn reflect_zero(&self) -> Option<Box<dyn Reflect>> {
                    Some(<Self as ReflectNew>::reflect_new())
                }

                fn reflect_apply(&mut self, value: &dyn Reflect) -> Result<(), ReflectError> {
                    let source = value.reflect_as_any().downcast_ref::<Self>().ok_or_else(|| {
                        ReflectError::ShapeMismatch {
                            operation: "reflect_apply",
                            target: type_name::<Self>().to_string(),
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
                    ReflectKind::Struct
                }
            }

            impl<$($t: Reflect + Clone + Default),+> ReflectStruct for ($($t,)+) {
                fn reflect_field_infos(&self) -> &'static [FieldInfo] {
                    // one static per monomorphized arity/types combination
                    static FIELDS: OnceLock<Vec<FieldInfo>> = OnceLock::new();
                    FIELDS.get_or_init(|| {
                        vec![
                            $(
                                FieldInfo {
                                    name: stringify!($idx),
                                    type_info: type_info::<$t>(),
                                },
                            )+
                        ]
                    })
                }

                fn reflect_field_at(&self, index: usize) -> Option<&dyn Reflect> {
                    match index {
                        $($idx => Some(&self.$idx as &dyn Reflect),)+
                        _ => None,
                    }
                }

                fn reflect_field_at_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
                    match index {
                        $($idx => Some(&mut self.$idx as &mut dyn Reflect),)+
                        _ => None,
                    }
                }
            }

            impl<$($t: Reflect + Clone + Default),+> ReflectNew for ($($t,)+) {
                fn reflect_new() -> Box<dyn Reflect> {
                    Box::new(($(<$t as Default>::default(),)+))
                }
            }
        )+
    };
}

impl_reflect_tuple! {
    (0 => A),
    (0 => A, 1 => B),
    (0 => A, 1 => B, 2 => C),
    (0 => A, 1 => B, 2 => C, 3 => D),
    (0 => A, 1 => B, 2 => C, 3 => D, 4 => E),
    (0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F),
    (0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G),
    (0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H),
    (0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I),
    (0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I, 9 => J),
    (0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I, 9 => J, 10 => K),
    (0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I, 9 => J, 10 => K, 11 => L),
    (0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I, 9 => J, 10 => K, 11 => L, 12 => M),
    (0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I, 9 => J, 10 => K, 11 => L, 12 => M, 13 => N),
    (0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I, 9 => J, 10 => K, 11 => L, 12 => M, 13 => N, 14 => O),
    (0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I, 9 => J, 10 => K, 11 => L, 12 => M, 13 => N, 14 => O, 15 => P),
}
