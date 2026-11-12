//! Reflect adapters: `impl Reflect` for `shared::Value` and std types, with
//! the full old-3 surface (clone / apply / as_any / kind / type info).
//! The only place in this crate that imports `dyn-rt-shared`.

use std::any::{Any, TypeId, type_name};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use dyn_rt_shared::Value;

use crate::domain::{
    FieldInfo, Reflect, ReflectEnum, ReflectIndirection, ReflectKind, ReflectMap, ReflectNew,
    ReflectRef, ReflectRefMut, ReflectSequence, TypeInfo, VariantInfo, type_info,
};
use crate::error::ReflectError;

fn unsupported(operation: &'static str, type_name: &str) -> ReflectError {
    ReflectError::UnsupportedOperation {
        operation,
        type_name: type_name.to_string(),
    }
}

/// Downcast a boxed reflected value into a concrete `Value`.
fn into_value(value: Box<dyn Reflect>) -> Result<Value, ReflectError> {
    let found = value.reflect_type_name().to_string();
    let any: Box<dyn Any> = value.reflect_into_any();
    any.downcast::<Value>()
        .map(|boxed| *boxed)
        .map_err(|_| ReflectError::ShapeMismatch {
            operation: "into_value",
            target: type_name::<Value>().to_string(),
            source: found,
        })
}

/// Extract a string key from an erased reflected key (`String`).
fn key_as_str(key: &dyn Reflect) -> Option<&str> {
    key.reflect_as_any()
        .downcast_ref::<String>()
        .map(|s| s.as_str())
}

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

impl ReflectMap for Value {
    fn reflect_get(&self, key: &dyn Reflect) -> Option<&dyn Reflect> {
        self.get(key_as_str(key)?)
            .map(|value| value as &dyn Reflect)
    }

    fn reflect_get_mut(&mut self, key: &dyn Reflect) -> Option<&mut dyn Reflect> {
        self.get_mut(key_as_str(key)?)
            .map(|value| value as &mut dyn Reflect)
    }

    fn reflect_set_key(
        &mut self,
        key: &dyn Reflect,
        value: Box<dyn Reflect>,
    ) -> Result<(), ReflectError> {
        let key = key_as_str(key)
            .ok_or_else(|| {
                unsupported("reflect_set_key (non-string key)", self.reflect_type_name())
            })?
            .to_string();
        let value = into_value(value)?;
        match self {
            Value::Object(entries) => {
                match entries.iter_mut().find(|(k, _)| *k == key) {
                    Some((_, slot)) => *slot = value,
                    None => entries.push((key, value)),
                }
                Ok(())
            }
            _ => Err(unsupported("reflect_set_key", self.reflect_type_name())),
        }
    }

    fn reflect_delete_key(&mut self, key: &dyn Reflect) -> Result<(), ReflectError> {
        let type_name = self.reflect_type_name().to_string();
        let key = key_as_str(key)
            .ok_or_else(|| unsupported("reflect_delete_key (non-string key)", &type_name))?
            .to_string();
        match self {
            Value::Object(entries) => {
                let position = entries.iter().position(|(k, _)| *k == key).ok_or_else(|| {
                    ReflectError::KeyNotFound {
                        key: key.clone(),
                        type_name: type_name.clone(),
                    }
                })?;
                entries.remove(position);
                Ok(())
            }
            _ => Err(unsupported("reflect_delete_key", &type_name)),
        }
    }

    fn reflect_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = (&'a dyn Reflect, &'a dyn Reflect)> + 'a> {
        match self.as_object() {
            Some(entries) => Box::new(
                entries
                    .iter()
                    .map(|(k, v)| (k as &dyn Reflect, v as &dyn Reflect)),
            ),
            None => Box::new(std::iter::empty()),
        }
    }

    fn reflect_length(&self) -> usize {
        self.as_object().map_or(0, |entries| entries.len())
    }

    fn reflect_key_type_info(&self) -> &'static TypeInfo {
        type_info::<String>()
    }

    fn reflect_value_type_info(&self) -> &'static TypeInfo {
        type_info::<Value>()
    }
}

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

/// Scalar impls for the primitive types — a declarative macro is the
/// sanctioned deduplication here (ADR 0005). All listed types are `Clone`.
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
    i64 => "i64",
    u64 => "u64",
    f64 => "f64",
    String => "string",
);

/// Structural clone of a `T` out of a boxed reflected value.
fn cloned_t<T: Reflect>(value: &dyn Reflect) -> T {
    let cloned: Box<dyn Any> = value.reflect_clone().reflect_into_any();
    *cloned
        .downcast::<T>()
        .expect("reflect_clone returns the same concrete type")
}

/// Downcast a boxed reflected value into a concrete `T`.
fn into_t<T: Reflect>(value: Box<dyn Reflect>) -> Result<T, ReflectError> {
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

impl<T: Reflect> Reflect for Box<T> {
    fn reflect_type_name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn reflect_ref(&self) -> ReflectRef<'_> {
        ReflectRef::Indirection(self)
    }

    fn reflect_ref_mut(&mut self) -> ReflectRefMut<'_> {
        ReflectRefMut::Indirection(self)
    }

    fn reflect_clone(&self) -> Box<dyn Reflect> {
        Box::new(Box::new(cloned_t::<T>(&**self)))
    }

    fn reflect_apply(&mut self, value: &dyn Reflect) -> Result<(), ReflectError> {
        if value.reflect_as_any().type_id() != TypeId::of::<Self>() {
            return Err(ReflectError::ShapeMismatch {
                operation: "reflect_apply",
                target: type_name::<Self>().to_string(),
                source: value.reflect_type_name().to_string(),
            });
        }
        **self = cloned_t::<T>(
            value
                .reflect_as_any()
                .downcast_ref::<Box<T>>()
                .expect("type id checked"),
        );
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
        ReflectKind::Indirection
    }
}

impl<T: Reflect> ReflectIndirection for Box<T> {
    fn reflect_get_inner(&self, callback: &mut dyn FnMut(Result<&dyn Reflect, ReflectError>)) {
        callback(Ok(&**self))
    }

    fn reflect_get_inner_mut(
        &mut self,
        callback: &mut dyn FnMut(Result<&mut dyn Reflect, ReflectError>),
    ) {
        callback(Ok(&mut **self))
    }

    fn reflect_set_inner(&mut self, value: Box<dyn Reflect>) -> Result<(), ReflectError> {
        **self = into_t::<T>(value)?;
        Ok(())
    }

    fn reflect_delete_inner(&mut self) -> Result<(), ReflectError> {
        Err(unsupported("reflect_delete_inner", type_name::<Self>()))
    }

    fn reflect_inner_type_info(&self) -> &'static TypeInfo {
        type_info::<T>()
    }
}

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

macro_rules! impl_reflect_shared_ptr {
    ($t:ty) => {
        impl<T: Reflect> Reflect for $t {
            fn reflect_type_name(&self) -> &'static str {
                type_name::<Self>()
            }

            fn reflect_ref(&self) -> ReflectRef<'_> {
                ReflectRef::Indirection(self)
            }

            fn reflect_ref_mut(&mut self) -> ReflectRefMut<'_> {
                ReflectRefMut::Indirection(self)
            }

            fn reflect_clone(&self) -> Box<dyn Reflect> {
                // share, not deep copy (per the module clone semantics)
                Box::new(self.clone())
            }

            fn reflect_apply(&mut self, value: &dyn Reflect) -> Result<(), ReflectError> {
                if value.reflect_as_any().type_id() != TypeId::of::<Self>() {
                    return Err(ReflectError::ShapeMismatch {
                        operation: "reflect_apply",
                        target: type_name::<Self>().to_string(),
                        source: value.reflect_type_name().to_string(),
                    });
                }
                *self = value
                    .reflect_as_any()
                    .downcast_ref::<Self>()
                    .expect("type id checked")
                    .clone();
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
                ReflectKind::Indirection
            }
        }

        impl<T: Reflect> ReflectIndirection for $t {
            fn reflect_get_inner(
                &self,
                callback: &mut dyn FnMut(Result<&dyn Reflect, ReflectError>),
            ) {
                callback(Ok(&**self))
            }

            fn reflect_get_inner_mut(
                &mut self,
                callback: &mut dyn FnMut(Result<&mut dyn Reflect, ReflectError>),
            ) {
                // shared pointers only expose &mut when uniquely owned
                match <$t>::get_mut(self) {
                    Some(inner) => callback(Ok(inner)),
                    None => callback(Err(ReflectError::InnerBorrowFailed {
                        type_name: type_name::<Self>().to_string(),
                    })),
                }
            }

            fn reflect_set_inner(&mut self, _value: Box<dyn Reflect>) -> Result<(), ReflectError> {
                // shared pointers cannot be written through without unique ownership
                Err(unsupported("reflect_set_inner", type_name::<Self>()))
            }

            fn reflect_delete_inner(&mut self) -> Result<(), ReflectError> {
                Err(unsupported("reflect_delete_inner", type_name::<Self>()))
            }

            fn reflect_inner_type_info(&self) -> &'static TypeInfo {
                type_info::<T>()
            }
        }
    };
}

impl_reflect_shared_ptr!(Rc<T>);
impl_reflect_shared_ptr!(Arc<T>);

impl<T: Reflect> Reflect for RefCell<T> {
    fn reflect_type_name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn reflect_ref(&self) -> ReflectRef<'_> {
        ReflectRef::Indirection(self)
    }

    fn reflect_ref_mut(&mut self) -> ReflectRefMut<'_> {
        ReflectRefMut::Indirection(self)
    }

    fn reflect_clone(&self) -> Box<dyn Reflect> {
        let inner = self.try_borrow().map(|b| cloned_t::<T>(&*b)).map_err(|_| {
            ReflectError::InnerBorrowFailed {
                type_name: type_name::<Self>().to_string(),
            }
        });
        match inner {
            Ok(value) => Box::new(RefCell::new(value)),
            Err(e) => panic!("reflect_clone on RefCell must not fail on a fresh borrow: {e}"),
        }
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
            .downcast_ref::<RefCell<T>>()
            .expect("type id checked");
        let cloned = source
            .try_borrow()
            .map(|b| cloned_t::<T>(&*b))
            .map_err(|_| ReflectError::InnerBorrowFailed {
                type_name: type_name::<Self>().to_string(),
            })?;
        *self = RefCell::new(cloned);
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
        ReflectKind::Indirection
    }
}

impl<T: Reflect> ReflectIndirection for RefCell<T> {
    fn reflect_get_inner(&self, callback: &mut dyn FnMut(Result<&dyn Reflect, ReflectError>)) {
        match self.try_borrow() {
            Ok(inner) => callback(Ok(&*inner)),
            Err(_) => callback(Err(ReflectError::InnerBorrowFailed {
                type_name: type_name::<Self>().to_string(),
            })),
        }
    }

    fn reflect_get_inner_mut(
        &mut self,
        callback: &mut dyn FnMut(Result<&mut dyn Reflect, ReflectError>),
    ) {
        match self.try_borrow_mut() {
            Ok(mut inner) => callback(Ok(&mut *inner)),
            Err(_) => callback(Err(ReflectError::InnerBorrowFailed {
                type_name: type_name::<Self>().to_string(),
            })),
        }
    }

    fn reflect_set_inner(&mut self, value: Box<dyn Reflect>) -> Result<(), ReflectError> {
        let value = into_t::<T>(value)?;
        match self.try_borrow_mut() {
            Ok(mut inner) => {
                *inner = value;
                Ok(())
            }
            Err(_) => Err(ReflectError::InnerBorrowFailed {
                type_name: type_name::<Self>().to_string(),
            }),
        }
    }

    fn reflect_delete_inner(&mut self) -> Result<(), ReflectError> {
        Err(unsupported("reflect_delete_inner", type_name::<Self>()))
    }

    fn reflect_inner_type_info(&self) -> &'static TypeInfo {
        type_info::<T>()
    }
}

impl<T: Reflect> Reflect for Mutex<T> {
    fn reflect_type_name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn reflect_ref(&self) -> ReflectRef<'_> {
        ReflectRef::Indirection(self)
    }

    fn reflect_ref_mut(&mut self) -> ReflectRefMut<'_> {
        ReflectRefMut::Indirection(self)
    }

    fn reflect_clone(&self) -> Box<dyn Reflect> {
        let guard = self.lock().expect("clone on unpoisoned mutex");
        Box::new(Mutex::new(cloned_t::<T>(&*guard)))
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
            .downcast_ref::<Mutex<T>>()
            .expect("type id checked");
        let guard = source.lock().map_err(|_| ReflectError::InnerBorrowFailed {
            type_name: type_name::<Self>().to_string(),
        })?;
        let cloned = cloned_t::<T>(&*guard);
        drop(guard);
        *self.get_mut().expect("mutex not poisoned") = cloned;
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
        ReflectKind::Indirection
    }
}

impl<T: Reflect> ReflectIndirection for Mutex<T> {
    fn reflect_get_inner(&self, callback: &mut dyn FnMut(Result<&dyn Reflect, ReflectError>)) {
        match self.lock() {
            Ok(guard) => callback(Ok(&*guard)),
            Err(_) => callback(Err(ReflectError::InnerBorrowFailed {
                type_name: type_name::<Self>().to_string(),
            })),
        }
    }

    fn reflect_get_inner_mut(
        &mut self,
        callback: &mut dyn FnMut(Result<&mut dyn Reflect, ReflectError>),
    ) {
        callback(Ok(self.get_mut().expect("mutex not poisoned")))
    }

    fn reflect_set_inner(&mut self, value: Box<dyn Reflect>) -> Result<(), ReflectError> {
        let value = into_t::<T>(value)?;
        *self.get_mut().expect("mutex not poisoned") = value;
        Ok(())
    }

    fn reflect_delete_inner(&mut self) -> Result<(), ReflectError> {
        Err(unsupported("reflect_delete_inner", type_name::<Self>()))
    }

    fn reflect_inner_type_info(&self) -> &'static TypeInfo {
        type_info::<T>()
    }
}

// FieldInfo is part of the domain surface; keep the import used even before
// any struct impl lands (derive crate will provide ReflectStruct impls).
#[allow(unused)]
fn _field_info_type_check(f: FieldInfo) -> &'static str {
    f.name
}
