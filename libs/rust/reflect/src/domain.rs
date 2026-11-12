//! Reflect domain: the full trait system, ported from dyn-rt-rs-old-3
//! (ADR 0008). std only — nothing here knows about `Value` or JSON.
//!
//! # Kind and access are one fact
//!
//! A value's kind is never queried independently of the operations it supports:
//! [`Reflect::reflect_ref`] returns the shape handle (e.g. `&dyn ReflectSequence`)
//! directly, so the kind of a value and the operations available on it cannot
//! disagree. Use [`ReflectRef::kind`] when only the classification is needed.
//!
//! All methods are prefixed with `reflect_` to avoid collisions with
//! user-defined methods.
//!
//! # Clone semantics
//!
//! Implementing [`Reflect`] implies a defined structural clone strategy:
//! scalars copy, containers recurse into their entries, `Box<T>` deep clones
//! its inner value, `Rc<T>`/`Arc<T>` clone the pointer (share, not deep copy),
//! and guarded wrappers (`RefCell<T>`, `Mutex<T>`) lock and clone their inner
//! value.
//!
//! # Writing values
//!
//! `Box<dyn Reflect>` parameters mean "move into a slot" — container insertion
//! methods take ownership. `&dyn Reflect` parameters mean "write into an
//! existing slot" — [`Reflect::reflect_apply`] copies into the receiver.

use std::any::{Any, TypeId, type_name};
use std::sync::OnceLock;

use crate::error::ReflectError;

/// The root gateway trait for runtime reflection.
///
/// Scalars implement only this trait; containers and nominal types additionally
/// implement the shape trait matching their structure.
pub trait Reflect: Any {
    /// Human-readable, stable type identifier.
    fn reflect_type_name(&self) -> &'static str;

    /// Routes this value into its structural sub-category (read-only). Kind
    /// and access cannot disagree — see the module docs.
    fn reflect_ref(&self) -> ReflectRef<'_>;

    /// Routes this value into its structural sub-category (mutable).
    fn reflect_ref_mut(&mut self) -> ReflectRefMut<'_>;

    /// Deep clones the underlying concrete type dynamically. Infallible by
    /// contract — see the module docs for per-wrapper clone semantics.
    fn reflect_clone(&self) -> Box<dyn Reflect>;

    /// Writes `value` into this one, in place, without taking ownership of
    /// `value`. Both sides must be the same concrete type; the copy is
    /// structural. Cross-shape conversion is a schema-layer concern.
    fn reflect_apply(&mut self, value: &dyn Reflect) -> Result<(), ReflectError>;

    /// Borrow this erased trait object as a concrete `&dyn Any`.
    fn reflect_as_any(&self) -> &dyn Any;

    /// Mutably borrow this erased trait object as a concrete `&mut dyn Any`.
    fn reflect_as_any_mut(&mut self) -> &mut dyn Any;

    /// Consume the box and unpack it back into an owned `Box<dyn Any>`.
    fn reflect_into_any(self: Box<Self>) -> Box<dyn Any>;

    /// Returns the structural kind of `T`, without an instance. The one
    /// type-level method on the trait.
    fn reflect_kind() -> ReflectKind
    where
        Self: Sized;
}

/// Immutable structural routing of a [`Reflect`] value. Matching on the enum
/// is the single dispatch point for traversal, serialization, schema work.
pub enum ReflectRef<'a> {
    /// Primitive values with no internal structure (e.g. `i64`, `bool`, `String`).
    Scalar(&'a dyn Reflect),
    /// Indexed collections (e.g. `Vec<T>`, `[T; N]`, `Box<[T]>`).
    Sequence(&'a dyn ReflectSequence),
    /// Dynamic maps with homogeneous key and value types (e.g. `HashMap<K, V>`).
    Map(&'a dyn ReflectMap),
    /// Static-keyed types with a fixed field set (structs).
    Struct(&'a dyn ReflectStruct),
    /// Types with named variants, possibly with payloads.
    Enum(&'a dyn ReflectEnum),
    /// Pointer-like types wrapping another value (e.g. `Box<T>`, `Rc<T>`, `RefCell<T>`).
    Indirection(&'a dyn ReflectIndirection),
}

impl ReflectRef<'_> {
    pub fn kind(&self) -> ReflectKind {
        match self {
            ReflectRef::Scalar(_) => ReflectKind::Scalar,
            ReflectRef::Sequence(_) => ReflectKind::Sequence,
            ReflectRef::Map(_) => ReflectKind::Map,
            ReflectRef::Struct(_) => ReflectKind::Struct,
            ReflectRef::Enum(_) => ReflectKind::Enum,
            ReflectRef::Indirection(_) => ReflectKind::Indirection,
        }
    }
}

/// Mutable structural routing of a [`Reflect`] value.
pub enum ReflectRefMut<'a> {
    Scalar(&'a mut dyn Reflect),
    Sequence(&'a mut dyn ReflectSequence),
    Map(&'a mut dyn ReflectMap),
    Struct(&'a mut dyn ReflectStruct),
    Enum(&'a mut dyn ReflectEnum),
    Indirection(&'a mut dyn ReflectIndirection),
}

impl ReflectRefMut<'_> {
    pub fn kind(&self) -> ReflectKind {
        match self {
            ReflectRefMut::Scalar(_) => ReflectKind::Scalar,
            ReflectRefMut::Sequence(_) => ReflectKind::Sequence,
            ReflectRefMut::Map(_) => ReflectKind::Map,
            ReflectRefMut::Struct(_) => ReflectKind::Struct,
            ReflectRefMut::Enum(_) => ReflectKind::Enum,
            ReflectRefMut::Indirection(_) => ReflectKind::Indirection,
        }
    }
}

/// Structural categories of reflectable types. Classification only — never a
/// dispatch input; dispatch happens by matching [`ReflectRef`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReflectKind {
    Scalar,
    Sequence,
    Map,
    Struct,
    Enum,
    Indirection,
}

impl std::fmt::Display for ReflectKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ReflectKind::Scalar => "scalar",
            ReflectKind::Sequence => "sequence",
            ReflectKind::Map => "map",
            ReflectKind::Struct => "struct",
            ReflectKind::Enum => "enum",
            ReflectKind::Indirection => "indirection",
        };
        f.write_str(name)
    }
}

/// Opt-in dynamic construction capability, implemented by types that can be
/// instantiated without an existing value (typically via `Default`).
pub trait ReflectNew: Reflect {
    /// Creates a new default instance of this type.
    fn reflect_new() -> Box<dyn Reflect>;
}

/// Metadata about a type: name, identity, structural category, and when
/// available a constructor.
#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub type_name: &'static str,
    pub type_id: TypeId,
    pub kind: ReflectKind,
    pub new_instance: Option<fn() -> Box<dyn Reflect>>,
}

impl TypeInfo {
    /// Creates a [`TypeInfo`] for any [`Reflect`] type (no constructor).
    pub fn of<T: Reflect>() -> Self {
        TypeInfo {
            type_name: type_name::<T>(),
            type_id: TypeId::of::<T>(),
            kind: T::reflect_kind(),
            new_instance: None,
        }
    }

    /// Creates a [`TypeInfo`] for a dynamically constructible type.
    pub fn of_constructible<T: ReflectNew>() -> Self {
        TypeInfo {
            type_name: type_name::<T>(),
            type_id: TypeId::of::<T>(),
            kind: T::reflect_kind(),
            new_instance: Some(T::reflect_new),
        }
    }

    /// Supplies (or replaces) the construction function.
    pub fn with_new_fn(mut self, new_fn: fn() -> Box<dyn Reflect>) -> Self {
        self.new_instance = Some(new_fn);
        self
    }

    /// Creates a new instance using the carried constructor, when present.
    pub fn construct(&self) -> Option<Box<dyn Reflect>> {
        let new_instance = self.new_instance?;
        Some(new_instance())
    }
}

/// Returns a cached, `'static` [`TypeInfo`] for any [`Reflect`] type. The cache
/// is per-monomorphization, so each concrete type is built exactly once.
pub fn type_info<T: Reflect>() -> &'static TypeInfo {
    static CACHE: OnceLock<TypeInfo> = OnceLock::new();
    CACHE.get_or_init(TypeInfo::of::<T>)
}

/// Static description of one field of a static-keyed type.
#[derive(Debug, Clone, Copy)]
pub struct FieldInfo {
    pub name: &'static str,
    pub type_info: &'static TypeInfo,
}

/// Static description of one variant of an enum type.
#[derive(Debug, Clone, Copy)]
pub struct VariantInfo {
    pub name: &'static str,
    pub payload_type_info: Option<&'static TypeInfo>,
}

/// Shape of a sequence (array/slice-like): positional access. Fixed-size
/// sequences implement `reflect_push` as an unsupported operation.
pub trait ReflectSequence: Reflect {
    /// Returns the element at `index`, or `None` when out of bounds.
    fn reflect_get(&self, index: usize) -> Option<&dyn Reflect>;

    /// Returns the number of elements.
    fn reflect_length(&self) -> usize;

    /// Replaces the element at `index`. Fails when out of bounds — appending
    /// is [`ReflectSequence::reflect_push`].
    fn reflect_set_index(
        &mut self,
        index: usize,
        value: Box<dyn Reflect>,
    ) -> Result<(), ReflectError>;

    /// Appends `value` to the end of the sequence.
    fn reflect_push(&mut self, value: Box<dyn Reflect>) -> Result<(), ReflectError>;

    /// Removes the element at `index`, shifting subsequent elements left.
    fn reflect_delete_index(&mut self, index: usize) -> Result<(), ReflectError>;

    /// Iterates over the elements in order.
    fn reflect_iter<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn Reflect> + 'a>;

    /// Type information for the (homogeneous) element type.
    fn reflect_element_type_info(&self) -> &'static TypeInfo;
}

/// Shape of a dynamic map: key-value structure with dynamic keys and one
/// uniform key and value type. Mirrors Go's `reflect.Map` accessors.
pub trait ReflectMap: Reflect {
    /// Returns the value at `key`, or `None` when the key is absent.
    fn reflect_get(&self, key: &dyn Reflect) -> Option<&dyn Reflect>;

    /// Returns the value at `key` mutably, or `None` when absent.
    fn reflect_get_mut(&mut self, key: &dyn Reflect) -> Option<&mut dyn Reflect>;

    /// Sets the value at `key`, inserting a new entry when the key is absent.
    fn reflect_set_key(
        &mut self,
        key: &dyn Reflect,
        value: Box<dyn Reflect>,
    ) -> Result<(), ReflectError>;

    /// Removes the entry at `key`. Errors with
    /// [`ReflectError::KeyNotFound`] when absent.
    fn reflect_delete_key(&mut self, key: &dyn Reflect) -> Result<(), ReflectError>;

    /// Iterates over `(key, value)` pairs.
    fn reflect_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = (&'a dyn Reflect, &'a dyn Reflect)> + 'a>;

    /// Returns the number of entries.
    fn reflect_length(&self) -> usize;

    /// Type information for the key type.
    fn reflect_key_type_info(&self) -> &'static TypeInfo;

    /// Type information for the value type.
    fn reflect_value_type_info(&self) -> &'static TypeInfo;

    /// Returns whether an entry exists at `key`.
    fn reflect_contains_key(&self, key: &dyn Reflect) -> bool {
        self.reflect_get(key).is_some()
    }

    /// Iterates over the keys only.
    fn reflect_iter_keys<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn Reflect> + 'a> {
        Box::new(self.reflect_iter().map(|(key, _)| key))
    }
}

/// Shape of a static-keyed type: a fixed set of named fields, each with its own
/// type. Mirrors Go's `reflect.Struct` accessors. Fields can neither be inserted
/// nor removed.
pub trait ReflectStruct: Reflect {
    /// Returns the fixed field set, in declaration order.
    fn reflect_field_infos(&self) -> &'static [FieldInfo];

    /// Returns the field at `index`, or `None` when out of bounds.
    fn reflect_field_at(&self, index: usize) -> Option<&dyn Reflect>;

    /// Returns the field at `index` mutably, or `None` when out of bounds.
    fn reflect_field_at_mut(&mut self, index: usize) -> Option<&mut dyn Reflect>;

    /// Returns the field named `name`, or `None` when no field has that name.
    fn reflect_field(&self, name: &str) -> Option<&dyn Reflect> {
        self.reflect_field_at(self.reflect_field_index(name)?)
    }

    /// Returns the field named `name` mutably, or `None`.
    fn reflect_field_mut(&mut self, name: &str) -> Option<&mut dyn Reflect> {
        let index = self.reflect_field_index(name)?;
        self.reflect_field_at_mut(index)
    }

    /// Returns the declaration index of the field named `name`, if present.
    fn reflect_field_index(&self, name: &str) -> Option<usize> {
        self.reflect_field_infos()
            .iter()
            .position(|field| field.name == name)
    }

    /// Type information for the field named `name`, if present.
    fn reflect_field_type_info(&self, name: &str) -> Option<&'static TypeInfo> {
        self.reflect_field_infos()
            .iter()
            .find(|field| field.name == name)
            .map(|field| field.type_info)
    }

    /// Returns the number of fields.
    fn reflect_field_count(&self) -> usize {
        self.reflect_field_infos().len()
    }

    /// Iterates over `(field name, value)` pairs in declaration order.
    fn reflect_iter_fields<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = (&'static str, &'a dyn Reflect)> + 'a> {
        let this: &'a Self = self;
        Box::new(
            this.reflect_field_infos()
                .iter()
                .enumerate()
                .filter_map(move |(index, field)| {
                    Some((field.name, this.reflect_field_at(index)?))
                }),
        )
    }
}

/// Shape of an enum with named variants, possibly with payloads.
pub trait ReflectEnum: Reflect {
    /// Returns the name of the active variant.
    fn reflect_active_variant(&self) -> &'static str;

    /// Returns the full variant table of the enum.
    fn reflect_variants(&self) -> &'static [VariantInfo];

    /// Returns the payload of the active variant, or `None` for unit variants.
    fn reflect_active_variant_payload(&self) -> Option<&dyn Reflect>;

    /// Returns the payload of the active variant mutably, or `None`.
    fn reflect_active_variant_payload_mut(&mut self) -> Option<&mut dyn Reflect>;

    /// Switches the enum to `variant_name`, with an optional payload. Fails
    /// when the variant does not exist or the payload does not match.
    fn reflect_set_variant(
        &mut self,
        variant_name: &str,
        payload: Option<Box<dyn Reflect>>,
    ) -> Result<(), ReflectError>;

    /// Type information for a variant's payload type, if it has one.
    fn reflect_variant_payload_type_info(&self, variant_name: &str) -> Option<&'static TypeInfo> {
        self.reflect_variants()
            .iter()
            .find(|variant| variant.name == variant_name)
            .and_then(|variant| variant.payload_type_info)
    }
}

/// Shape of an indirection: pointers, references and smart pointers wrapping
/// another value. Reads are callback-shaped because guard-based wrappers
/// (`RefCell<T>`, `Mutex<T>`) only borrow their inner value inside the callback
/// scope. Implementations must invoke the callback exactly once per call.
pub trait ReflectIndirection: Reflect {
    /// Passes the inner value to `callback`, or the reason it cannot be borrowed.
    fn reflect_get_inner(&self, callback: &mut dyn FnMut(Result<&dyn Reflect, ReflectError>));

    /// Passes the inner value mutably to `callback`, or the reason.
    fn reflect_get_inner_mut(
        &mut self,
        callback: &mut dyn FnMut(Result<&mut dyn Reflect, ReflectError>),
    );

    /// Replaces the inner value.
    fn reflect_set_inner(&mut self, value: Box<dyn Reflect>) -> Result<(), ReflectError>;

    /// Empties the indirection, when the wrapper supports it (e.g. `Option<T>`
    /// set to `None`). Unsupported for always-full wrappers like `Box<T>`.
    fn reflect_delete_inner(&mut self) -> Result<(), ReflectError>;

    /// Type information for the wrapped type.
    fn reflect_inner_type_info(&self) -> &'static TypeInfo;
}
