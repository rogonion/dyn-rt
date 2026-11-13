//! Indirection adapters: `Box<T>`, `Rc<T>`, `Arc<T>`, `RefCell<T>`, `Mutex<T>`.

use std::any::{Any, TypeId, type_name};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use super::{cloned_t, into_t, unsupported};
use crate::domain::{
    Reflect, ReflectIndirection, ReflectKind, ReflectNew, ReflectRef, ReflectRefMut, TypeInfo,
    type_info,
};
use crate::error::ReflectError;

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

    fn reflect_inner_ref(&self) -> Option<&dyn Reflect> {
        Some(&**self)
    }

    fn reflect_inner_mut(&mut self) -> Option<&mut dyn Reflect> {
        Some(&mut **self)
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

/// Shared-pointer adapters (`Rc<T>`, `Arc<T>`): clone the pointer (share, not
/// deep copy); mutable inner access only when uniquely owned; no set/delete
/// through a shared pointer.
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

            fn reflect_inner_ref(&self) -> Option<&dyn Reflect> {
                Some(&**self)
            }

            fn reflect_inner_mut(&mut self) -> Option<&mut dyn Reflect> {
                <$t>::get_mut(self).map(|inner| inner as &mut dyn Reflect)
            }

            fn reflect_get_inner_mut(
                &mut self,
                callback: &mut dyn FnMut(Result<&mut dyn Reflect, ReflectError>),
            ) {
                match <$t>::get_mut(self) {
                    Some(inner) => callback(Ok(inner)),
                    None => callback(Err(ReflectError::InnerBorrowFailed {
                        type_name: type_name::<Self>().to_string(),
                    })),
                }
            }

            fn reflect_set_inner(&mut self, _value: Box<dyn Reflect>) -> Result<(), ReflectError> {
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
        let inner = self
            .try_borrow()
            .expect("reflect_clone on RefCell requires no active borrow");
        Box::new(RefCell::new(cloned_t::<T>(&*inner)))
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
        let borrowed = source
            .try_borrow()
            .map_err(|_| ReflectError::InnerBorrowFailed {
                type_name: type_name::<Self>().to_string(),
            })?;
        let cloned = cloned_t::<T>(&*borrowed);
        drop(borrowed);
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

/// Pointer-like wrappers are constructible when the inner type is `Default`.
macro_rules! impl_reflect_new_indirection {
    ($t:ty, $ctor:expr) => {
        impl<T: Reflect + Default> ReflectNew for $t {
            fn reflect_new() -> Box<dyn Reflect> {
                Box::new($ctor(T::default()))
            }
        }
    };
}

impl_reflect_new_indirection!(Box<T>, Box::new);
impl_reflect_new_indirection!(Rc<T>, Rc::new);
impl_reflect_new_indirection!(Arc<T>, Arc::new);

impl<T: Reflect + Default> ReflectNew for RefCell<T> {
    fn reflect_new() -> Box<dyn Reflect> {
        Box::new(RefCell::new(T::default()))
    }
}

impl<T: Reflect + Default> ReflectNew for Mutex<T> {
    fn reflect_new() -> Box<dyn Reflect> {
        Box::new(Mutex::new(T::default()))
    }
}
