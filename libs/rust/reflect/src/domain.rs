//! Reflect domain: the trait system. std only — nothing here knows about
//! `Value` or JSON.
//!
//! Design (from dyn-rt-rs-old-3, ADR 0008): kind and access are ONE fact.
//! [`Reflect::reflect_ref`] returns the shape handle directly, so a value's
//! kind and the operations available on it cannot disagree. Use
//! [`ReflectRef::kind`] when only the classification is needed.
//!
//! All methods are prefixed with `reflect_` to avoid collisions with
//! user-defined methods.

use std::any::Any;

use crate::error::ReflectError;

/// Classification of a reflectable value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Scalar,
    Sequence,
    Map,
    Indirection,
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Kind::Scalar => "scalar",
            Kind::Sequence => "sequence",
            Kind::Map => "map",
            Kind::Indirection => "indirection",
        };
        f.write_str(name)
    }
}

/// The root gateway trait for runtime reflection. Any type participating in
/// the dynamic plane implements this; containers additionally implement the
/// shape trait matching their structure.
pub trait Reflect: Any {
    /// Human-readable, stable type identifier.
    fn reflect_type_name(&self) -> &'static str;

    /// Routes this value into its structural sub-category (read-only).
    fn reflect_ref(&self) -> ReflectRef<'_>;

    /// Routes this value into its structural sub-category (mutable).
    fn reflect_ref_mut(&mut self) -> ReflectRefMut<'_>;
}

/// A routed read-only view of a reflectable value.
pub enum ReflectRef<'a> {
    Scalar,
    Sequence(&'a dyn ReflectSequence),
    Map(&'a dyn ReflectMap),
    Indirection(&'a dyn ReflectIndirection),
}

impl ReflectRef<'_> {
    pub fn kind(&self) -> Kind {
        match self {
            ReflectRef::Scalar => Kind::Scalar,
            ReflectRef::Sequence(_) => Kind::Sequence,
            ReflectRef::Map(_) => Kind::Map,
            ReflectRef::Indirection(_) => Kind::Indirection,
        }
    }
}

/// A routed mutable view of a reflectable value.
pub enum ReflectRefMut<'a> {
    Scalar,
    Sequence(&'a mut dyn ReflectSequence),
    Map(&'a mut dyn ReflectMap),
    Indirection(&'a mut dyn ReflectIndirection),
}

impl ReflectRefMut<'_> {
    pub fn kind(&self) -> Kind {
        match self {
            ReflectRefMut::Scalar => Kind::Scalar,
            ReflectRefMut::Sequence(_) => Kind::Sequence,
            ReflectRefMut::Map(_) => Kind::Map,
            ReflectRefMut::Indirection(_) => Kind::Indirection,
        }
    }
}

/// Shape of a sequence (array/slice-like): positional access.
pub trait ReflectSequence: Reflect {
    fn reflect_len(&self) -> usize;

    fn reflect_is_empty(&self) -> bool {
        self.reflect_len() == 0
    }

    fn reflect_get(&self, index: usize) -> Option<&dyn Reflect>;

    fn reflect_get_mut(&mut self, index: usize) -> Option<&mut dyn Reflect>;

    /// Replace the element at `index`. Errors with
    /// [`ReflectError::IndexOutOfBounds`] when out of range.
    fn reflect_set(&mut self, index: usize, value: Box<dyn Reflect>) -> Result<(), ReflectError>;

    /// Append an element.
    fn reflect_push(&mut self, value: Box<dyn Reflect>) -> Result<(), ReflectError>;

    /// All elements in order.
    fn reflect_iter(&self) -> Vec<&dyn Reflect>;
}

/// Shape of a map-like value: string-keyed access with dynamic keys.
pub trait ReflectMap: Reflect {
    fn reflect_len(&self) -> usize;

    fn reflect_is_empty(&self) -> bool {
        self.reflect_len() == 0
    }

    fn reflect_contains_key(&self, key: &str) -> bool;

    fn reflect_get(&self, key: &str) -> Option<&dyn Reflect>;

    fn reflect_get_mut(&mut self, key: &str) -> Option<&mut dyn Reflect>;

    /// Insert or replace the value at `key`.
    fn reflect_set(&mut self, key: &str, value: Box<dyn Reflect>) -> Result<(), ReflectError>;

    /// Remove and return the value at `key`, if present.
    fn reflect_delete(&mut self, key: &str) -> Option<Box<dyn Reflect>>;

    /// All keys in iteration (insertion) order.
    fn reflect_keys(&self) -> Vec<String>;

    /// All entries in iteration (insertion) order.
    fn reflect_iter(&self) -> Vec<(&str, &dyn Reflect)>;
}

/// Shape of an indirection (Box/Rc-like wrapper): the inner value can only be
/// borrowed inside a callback scope, hence the callback-shaped reads.
pub trait ReflectIndirection: Reflect {
    fn reflect_get_inner(&self, on_inner: &mut dyn FnMut(Result<&dyn Reflect, ReflectError>));

    fn reflect_get_inner_mut(
        &mut self,
        on_inner: &mut dyn FnMut(Result<&mut dyn Reflect, ReflectError>),
    );
}
