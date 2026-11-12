//! dyn-rt reflect module — general-purpose runtime reflection for Rust
//! (the go-`reflect` analogue), porting the full dyn-rt-rs-old-3 surface
//! (ADR 0008). Hexagon inside one crate (ADR 0007):
//!
//! - `domain`: the `Reflect` trait system — std only, knows nothing about JSON
//! - `application`: thin semantic helpers (kind_of, as_* downcasts, children,
//!   reflect_with_inner*)
//! - `adapters`: `impl Reflect` for `shared::Value`, primitives, `Vec<T>`,
//!   `Box<T>`, `Option<T>`, `Rc`/`Arc`/`RefCell`/`Mutex` — the only place
//!   importing `dyn-rt-shared`

pub mod adapters;
pub mod application;
pub mod domain;
pub mod error;

pub use application::{
    as_enum, as_indirection, as_map, as_map_mut, as_sequence, as_sequence_mut, as_struct, children,
    kind_of, reflect_with_inner, reflect_with_inner_mut,
};
pub use domain::{
    FieldInfo, Reflect, ReflectEnum, ReflectIndirection, ReflectKind, ReflectMap, ReflectNew,
    ReflectRef, ReflectRefMut, ReflectSequence, ReflectStruct, TypeInfo, VariantInfo, type_info,
};
pub use error::ReflectError;
