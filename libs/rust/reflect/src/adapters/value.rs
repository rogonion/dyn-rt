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

    fn reflect_scalar_i64(&self) -> Option<i64> {
        match self {
            Value::Number(dyn_rt_shared::Number::PosInt(v)) => i64::try_from(*v).ok(),
            Value::Number(dyn_rt_shared::Number::NegInt(v)) => Some(*v),
            _ => None,
        }
    }

    fn reflect_scalar_f64(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(n.as_f64()),
            _ => None,
        }
    }

    fn reflect_scalar_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    fn reflect_scalar_string(&self) -> Option<String> {
        match self {
            Value::Null => Some("null".to_string()),
            Value::Bool(b) => Some(b.to_string()),
            Value::Number(n) => Some(match n {
                dyn_rt_shared::Number::PosInt(v) => v.to_string(),
                dyn_rt_shared::Number::NegInt(v) => v.to_string(),
                dyn_rt_shared::Number::Float(v) => v.to_string(),
            }),
            Value::String(s) => Some(s.clone()),
            _ => None,
        }
    }
}

impl ReflectNew for Value {
    fn reflect_new() -> Box<dyn Reflect> {
        Box::new(Value::default())
    }
}
