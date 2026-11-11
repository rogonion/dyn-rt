//! Reflect adapters: `impl Reflect` for `shared::Value` and std types.
//! The only place in this crate that imports `dyn-rt-shared`.

use std::any::Any;

use dyn_rt_shared::Value;

use crate::domain::{
    Reflect, ReflectIndirection, ReflectMap, ReflectRef, ReflectRefMut, ReflectSequence,
};
use crate::error::ReflectError;

/// Downcast a boxed reflected value into a concrete `Value`.
fn into_value(value: Box<dyn Reflect>) -> Result<Value, ReflectError> {
    let found = value.reflect_type_name().to_string();
    let any: Box<dyn Any> = value; // trait upcasting: Reflect: Any
    any.downcast::<Value>()
        .map(|boxed| *boxed)
        .map_err(|_| ReflectError::TypeMismatch {
            expected: "dyn-rt Value",
            found,
        })
}

impl Reflect for Value {
    fn reflect_type_name(&self) -> &'static str {
        self.type_name()
    }

    fn reflect_ref(&self) -> ReflectRef<'_> {
        match self {
            Value::Object(_) => ReflectRef::Map(self),
            Value::Array(_) => ReflectRef::Sequence(self),
            _ => ReflectRef::Scalar,
        }
    }

    fn reflect_ref_mut(&mut self) -> ReflectRefMut<'_> {
        match self {
            Value::Object(_) => ReflectRefMut::Map(self),
            Value::Array(_) => ReflectRefMut::Sequence(self),
            _ => ReflectRefMut::Scalar,
        }
    }
}

impl ReflectMap for Value {
    fn reflect_len(&self) -> usize {
        self.as_object().map_or(0, |entries| entries.len())
    }

    fn reflect_contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    fn reflect_get(&self, key: &str) -> Option<&dyn Reflect> {
        self.get(key).map(|value| value as &dyn Reflect)
    }

    fn reflect_get_mut(&mut self, key: &str) -> Option<&mut dyn Reflect> {
        self.get_mut(key).map(|value| value as &mut dyn Reflect)
    }

    fn reflect_set(&mut self, key: &str, value: Box<dyn Reflect>) -> Result<(), ReflectError> {
        let value = into_value(value)?;
        match self {
            Value::Object(entries) => {
                match entries.iter_mut().find(|(k, _)| k == key) {
                    Some((_, slot)) => *slot = value,
                    None => entries.push((key.to_string(), value)),
                }
                Ok(())
            }
            _ => Err(ReflectError::UnsupportedOperation {
                operation: "reflect_set",
                type_name: self.reflect_type_name().to_string(),
            }),
        }
    }

    fn reflect_delete(&mut self, key: &str) -> Option<Box<dyn Reflect>> {
        let entries = self.as_object_mut()?;
        let position = entries.iter().position(|(k, _)| k == key)?;
        let (_, removed) = entries.remove(position);
        Some(Box::new(removed))
    }

    fn reflect_keys(&self) -> Vec<String> {
        self.as_object()
            .map(|entries| entries.iter().map(|(k, _)| k.clone()).collect())
            .unwrap_or_default()
    }

    fn reflect_iter(&self) -> Vec<(&str, &dyn Reflect)> {
        self.as_object()
            .map(|entries| {
                entries
                    .iter()
                    .map(|(k, v)| (k.as_str(), v as &dyn Reflect))
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl ReflectSequence for Value {
    fn reflect_len(&self) -> usize {
        self.as_array().map_or(0, |items| items.len())
    }

    fn reflect_get(&self, index: usize) -> Option<&dyn Reflect> {
        self.as_array()?
            .get(index)
            .map(|value| value as &dyn Reflect)
    }

    fn reflect_get_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        self.as_array_mut()?
            .get_mut(index)
            .map(|value| value as &mut dyn Reflect)
    }

    fn reflect_set(&mut self, index: usize, value: Box<dyn Reflect>) -> Result<(), ReflectError> {
        let value = into_value(value)?;
        match self {
            Value::Array(items) => {
                if index >= items.len() {
                    return Err(ReflectError::IndexOutOfBounds {
                        index,
                        len: items.len(),
                    });
                }
                items[index] = value;
                Ok(())
            }
            _ => Err(ReflectError::UnsupportedOperation {
                operation: "reflect_set",
                type_name: self.reflect_type_name().to_string(),
            }),
        }
    }

    fn reflect_push(&mut self, value: Box<dyn Reflect>) -> Result<(), ReflectError> {
        let value = into_value(value)?;
        match self {
            Value::Array(items) => {
                items.push(value);
                Ok(())
            }
            _ => Err(ReflectError::UnsupportedOperation {
                operation: "reflect_push",
                type_name: self.reflect_type_name().to_string(),
            }),
        }
    }

    fn reflect_iter(&self) -> Vec<&dyn Reflect> {
        self.as_array()
            .map(|items| items.iter().map(|value| value as &dyn Reflect).collect())
            .unwrap_or_default()
    }
}

/// Scalar impls for the primitive types — a declarative macro is the
/// sanctioned deduplication here (ADR 0005).
macro_rules! impl_reflect_scalar {
    ($($t:ty => $name:literal),* $(,)?) => {
        $(
            impl Reflect for $t {
                fn reflect_type_name(&self) -> &'static str {
                    $name
                }

                fn reflect_ref(&self) -> ReflectRef<'_> {
                    ReflectRef::Scalar
                }

                fn reflect_ref_mut(&mut self) -> ReflectRefMut<'_> {
                    ReflectRefMut::Scalar
                }
            }
        )*
    };
}

impl_reflect_scalar!(
    bool => "bool",
    i64 => "i64",
    u64 => "u64",
    f64 => "f64",
    String => "string",
);

impl<T: Reflect> Reflect for Vec<T> {
    fn reflect_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn reflect_ref(&self) -> ReflectRef<'_> {
        ReflectRef::Sequence(self)
    }

    fn reflect_ref_mut(&mut self) -> ReflectRefMut<'_> {
        ReflectRefMut::Sequence(self)
    }
}

impl<T: Reflect> ReflectSequence for Vec<T> {
    fn reflect_len(&self) -> usize {
        self.len()
    }

    fn reflect_get(&self, index: usize) -> Option<&dyn Reflect> {
        self.get(index).map(|value| value as &dyn Reflect)
    }

    fn reflect_get_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        self.get_mut(index).map(|value| value as &mut dyn Reflect)
    }

    fn reflect_set(&mut self, index: usize, value: Box<dyn Reflect>) -> Result<(), ReflectError> {
        let found = value.reflect_type_name().to_string();
        let any: Box<dyn Any> = value;
        let value = any
            .downcast::<T>()
            .map_err(|_| ReflectError::TypeMismatch {
                expected: std::any::type_name::<T>(),
                found,
            })?;
        if index >= self.len() {
            return Err(ReflectError::IndexOutOfBounds {
                index,
                len: self.len(),
            });
        }
        self[index] = *value;
        Ok(())
    }

    fn reflect_push(&mut self, value: Box<dyn Reflect>) -> Result<(), ReflectError> {
        let found = value.reflect_type_name().to_string();
        let any: Box<dyn Any> = value;
        let value = any
            .downcast::<T>()
            .map_err(|_| ReflectError::TypeMismatch {
                expected: std::any::type_name::<T>(),
                found,
            })?;
        self.push(*value);
        Ok(())
    }

    fn reflect_iter(&self) -> Vec<&dyn Reflect> {
        self.iter().map(|value| value as &dyn Reflect).collect()
    }
}

impl<T: Reflect> Reflect for Box<T> {
    fn reflect_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn reflect_ref(&self) -> ReflectRef<'_> {
        ReflectRef::Indirection(self)
    }

    fn reflect_ref_mut(&mut self) -> ReflectRefMut<'_> {
        ReflectRefMut::Indirection(self)
    }
}

impl<T: Reflect> ReflectIndirection for Box<T> {
    fn reflect_get_inner(&self, on_inner: &mut dyn FnMut(Result<&dyn Reflect, ReflectError>)) {
        on_inner(Ok(&**self))
    }

    fn reflect_get_inner_mut(
        &mut self,
        on_inner: &mut dyn FnMut(Result<&mut dyn Reflect, ReflectError>),
    ) {
        on_inner(Ok(&mut **self))
    }
}
