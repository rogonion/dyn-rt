//! Enum adapters: `Option<T>` as an enum with `Some`/`None` variants.

use std::any::{Any, TypeId, type_name};

use super::{cloned_t, into_t};
use crate::domain::{
    Reflect, ReflectEnum, ReflectKind, ReflectNew, ReflectRef, ReflectRefMut, VariantInfo,
    type_info,
};
use crate::error::ReflectError;

/// The `Some`/`None` variant table for `Option<T>` (per-monomorphization cache).
fn option_variants<T: Reflect>() -> &'static [VariantInfo] {
    static CACHE: std::sync::OnceLock<Vec<VariantInfo>> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| {
        vec![
            VariantInfo {
                name: "Some",
                payload_type_info: Some(type_info::<T>()),
            },
            VariantInfo {
                name: "None",
                payload_type_info: None,
            },
        ]
    })
}

impl<T: Reflect> Reflect for Option<T> {
    fn reflect_type_name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn reflect_ref(&self) -> ReflectRef<'_> {
        ReflectRef::Enum(self)
    }

    fn reflect_ref_mut(&mut self) -> ReflectRefMut<'_> {
        ReflectRefMut::Enum(self)
    }

    fn reflect_clone(&self) -> Box<dyn Reflect> {
        Box::new(self.as_ref().map(|v| cloned_t::<T>(v)))
    }

    fn reflect_apply(&mut self, value: &dyn Reflect) -> Result<(), ReflectError> {
        if value.reflect_as_any().type_id() != TypeId::of::<Self>() {
            return Err(ReflectError::ShapeMismatch {
                operation: "reflect_apply",
                target: type_name::<Self>().to_string(),
                source: value.reflect_type_name().to_string(),
            });
        }
        let source = value
            .reflect_as_any()
            .downcast_ref::<Option<T>>()
            .expect("type id checked");
        *self = source.as_ref().map(|v| cloned_t::<T>(v));
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
        ReflectKind::Enum
    }
}

impl<T: Reflect> ReflectEnum for Option<T> {
    fn reflect_active_variant(&self) -> &'static str {
        match self {
            Some(_) => "Some",
            None => "None",
        }
    }

    fn reflect_variants(&self) -> &'static [VariantInfo] {
        option_variants::<T>()
    }

    fn reflect_active_variant_payload(&self) -> Option<&dyn Reflect> {
        self.as_ref().map(|value| value as &dyn Reflect)
    }

    fn reflect_active_variant_payload_mut(&mut self) -> Option<&mut dyn Reflect> {
        self.as_mut().map(|value| value as &mut dyn Reflect)
    }

    fn reflect_set_variant(
        &mut self,
        variant_name: &str,
        payload: Option<Box<dyn Reflect>>,
    ) -> Result<(), ReflectError> {
        match variant_name {
            "Some" => {
                let payload = payload.ok_or_else(|| ReflectError::UnknownVariant {
                    variant: "Some (missing payload)".to_string(),
                    type_name: type_name::<Self>().to_string(),
                })?;
                *self = Some(into_t::<T>(payload)?);
                Ok(())
            }
            "None" => {
                *self = None;
                Ok(())
            }
            other => Err(ReflectError::UnknownVariant {
                variant: other.to_string(),
                type_name: type_name::<Self>().to_string(),
            }),
        }
    }
}

/// `Option<T>` is constructible as `None`.
impl<T: Reflect> ReflectNew for Option<T> {
    fn reflect_new() -> Box<dyn Reflect> {
        Box::new(None::<T>)
    }
}
