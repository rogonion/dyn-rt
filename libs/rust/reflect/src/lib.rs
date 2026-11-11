//! dyn-rt reflect module — general-purpose runtime reflection for Rust
//! (the go-`reflect` analogue). Hexagon inside one crate (ADR 0007/0008):
//!
//! - `domain`: the `Reflect` trait system — std only, knows nothing about JSON
//! - `application`: thin semantic helpers (kind_of, as_map/as_sequence, children)
//! - `adapters`: `impl Reflect` for `shared::Value`, primitives, `Vec<T>`,
//!   `Box<T>` — the only place importing `dyn-rt-shared`

pub mod adapters;
pub mod application;
pub mod domain;
pub mod error;

pub use application::{as_map, as_map_mut, as_sequence, as_sequence_mut, children, kind_of};
pub use domain::{
    Kind, Reflect, ReflectIndirection, ReflectMap, ReflectRef, ReflectRefMut, ReflectSequence,
};
pub use error::ReflectError;
