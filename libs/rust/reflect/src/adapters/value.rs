//! The base `Reflect` impl for `shared::Value` (routing by variant) plus
//! `ReflectNew`. The Map/Sequence impls for `Value` live in `map.rs` /
//! `sequence.rs`.

use std::any::{Any, TypeId, type_name};

use dyn_rt_shared::Value;

use crate::domain::{Reflect, ReflectKind, ReflectNew, ReflectRef, ReflectRefMut};
use crate::error::ReflectError;

impl Reflect for Value {
    fn reflect_type_name(&self) -> &'static str {
        self.type_name()
    }

    fn reflect_ref(&self) -> ReflectRef<'_> {
        match self {
            Value::Object(_) => ReflectRef::Map(self),
            Value::Array(_) => ReflectRef::Sequence(self),
            _ => ReflectRef::Scalar(self),
        }
    }

    fn reflect_ref_mut(&mut self) -> ReflectRefMut<'_> {
        match self {
            Value::Object(_) => ReflectRefMut::Map(self),
            Value::Array(_) => ReflectRefMut::Sequence(self),
            _ => ReflectRefMut::Scalar(self),
        }
    }

    fn reflect_clone(&self) -> Box<dyn Reflect> {
        Box::new(self.clone())
    }

    fn reflect_apply(&mut self, value: &dyn Reflect) -> Result<(), ReflectError> {
        if value.reflect_as_any().type_id() != TypeId::of::<Value>() {
            return Err(ReflectError::ShapeMismatch {
                operation: "reflect_apply",
                target: type_name::<Value>().to_string(),
                source: value.reflect_type_name().to_string(),
            });
        }
        let source = value
            .reflect_as_any()
            .downcast_ref::<Value>()
            .expect("type id checked");
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
        // Value is a nominal enum at the type level; instances route by variant.
        ReflectKind::Enum
    }
}

impl ReflectNew for Value {
    fn reflect_new() -> Box<dyn Reflect> {
        Box::new(Value::default())
    }
}
