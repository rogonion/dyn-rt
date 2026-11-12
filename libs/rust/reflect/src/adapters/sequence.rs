//! Sequence adapters: `ReflectSequence` for `shared::Value` arrays, and
//! `Vec<T>` as a generic sequence.

use std::any::{Any, TypeId, type_name};

use dyn_rt_shared::Value;

use super::{cloned_t, into_t, into_value, unsupported};
use crate::domain::{
    Reflect, ReflectKind, ReflectNew, ReflectRef, ReflectRefMut, ReflectSequence, TypeInfo,
    type_info,
};
use crate::error::ReflectError;

impl ReflectSequence for Value {
    fn reflect_get(&self, index: usize) -> Option<&dyn Reflect> {
        self.as_array()?
            .get(index)
            .map(|value| value as &dyn Reflect)
    }

    fn reflect_length(&self) -> usize {
        self.as_array().map_or(0, |items| items.len())
    }

    fn reflect_set_index(
        &mut self,
        index: usize,
        value: Box<dyn Reflect>,
    ) -> Result<(), ReflectError> {
        let value = into_value(value)?;
        match self {
            Value::Array(items) => {
                if index >= items.len() {
                    return Err(ReflectError::IndexOutOfBounds {
                        index,
                        length: items.len(),
                        type_name: self.reflect_type_name().to_string(),
                    });
                }
                items[index] = value;
                Ok(())
            }
            _ => Err(unsupported("reflect_set_index", self.reflect_type_name())),
        }
    }

    fn reflect_push(&mut self, value: Box<dyn Reflect>) -> Result<(), ReflectError> {
        let value = into_value(value)?;
        match self {
            Value::Array(items) => {
                items.push(value);
                Ok(())
            }
            _ => Err(unsupported("reflect_push", self.reflect_type_name())),
        }
    }

    fn reflect_delete_index(&mut self, index: usize) -> Result<(), ReflectError> {
        match self {
            Value::Array(items) => {
                if index >= items.len() {
                    return Err(ReflectError::IndexOutOfBounds {
                        index,
                        length: items.len(),
                        type_name: self.reflect_type_name().to_string(),
                    });
                }
                items.remove(index);
                Ok(())
            }
            _ => Err(unsupported(
                "reflect_delete_index",
                self.reflect_type_name(),
            )),
        }
    }

    fn reflect_iter<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn Reflect> + 'a> {
        match self.as_array() {
            Some(items) => Box::new(items.iter().map(|value| value as &dyn Reflect)),
            None => Box::new(std::iter::empty()),
        }
    }

    fn reflect_element_type_info(&self) -> &'static TypeInfo {
        type_info::<Value>()
    }
}

impl<T: Reflect> Reflect for Vec<T> {
    fn reflect_type_name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn reflect_ref(&self) -> ReflectRef<'_> {
        ReflectRef::Sequence(self)
    }

    fn reflect_ref_mut(&mut self) -> ReflectRefMut<'_> {
        ReflectRefMut::Sequence(self)
    }

    fn reflect_clone(&self) -> Box<dyn Reflect> {
        Box::new(self.iter().map(|v| cloned_t::<T>(v)).collect::<Vec<T>>())
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
            .downcast_ref::<Vec<T>>()
            .expect("type id checked");
        *self = source.iter().map(|v| cloned_t::<T>(v)).collect();
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
        ReflectKind::Sequence
    }
}

impl<T: Reflect> ReflectSequence for Vec<T> {
    fn reflect_get(&self, index: usize) -> Option<&dyn Reflect> {
        self.get(index).map(|value| value as &dyn Reflect)
    }

    fn reflect_length(&self) -> usize {
        self.len()
    }

    fn reflect_set_index(
        &mut self,
        index: usize,
        value: Box<dyn Reflect>,
    ) -> Result<(), ReflectError> {
        if index >= self.len() {
            return Err(ReflectError::IndexOutOfBounds {
                index,
                length: self.len(),
                type_name: type_name::<Self>().to_string(),
            });
        }
        self[index] = into_t::<T>(value)?;
        Ok(())
    }

    fn reflect_push(&mut self, value: Box<dyn Reflect>) -> Result<(), ReflectError> {
        self.push(into_t::<T>(value)?);
        Ok(())
    }

    fn reflect_delete_index(&mut self, index: usize) -> Result<(), ReflectError> {
        if index >= self.len() {
            return Err(ReflectError::IndexOutOfBounds {
                index,
                length: self.len(),
                type_name: type_name::<Self>().to_string(),
            });
        }
        self.remove(index);
        Ok(())
    }

    fn reflect_iter<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn Reflect> + 'a> {
        Box::new(self.iter().map(|value| value as &dyn Reflect))
    }

    fn reflect_element_type_info(&self) -> &'static TypeInfo {
        type_info::<T>()
    }
}

/// `Vec<T>` is constructible as an empty vec.
impl<T: Reflect> ReflectNew for Vec<T> {
    fn reflect_new() -> Box<dyn Reflect> {
        Box::new(Vec::<T>::new())
    }
}
