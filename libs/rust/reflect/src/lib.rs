//! dyn-rt reflect module — general-purpose runtime reflection for Rust
//! (the go-`reflect` analogue), porting the full dyn-rt-rs-old-3 surface
//! (ADR 0008). Hexagon inside one crate (ADR 0007):
//!
//! - `domain.rs`: the whole trait system — std only, knows nothing about
//!   JSON (`ReflectKind`, `Reflect`, `ReflectRef`/`ReflectRefMut`, `TypeInfo`,
//!   and the six shape traits)
//! - `application.rs`: thin semantic helpers (`kind_of`, `children`, one
//!   `as_*` downcast pair per kind, `reflect_with_inner*`)
//! - `adapters/`: `impl Reflect` for `shared::Value` and std types, one file
//!   per `ReflectKind` so the folder listing shows which built-in types are
//!   covered — the only place importing `dyn-rt-shared`

pub mod adapters;
pub mod application;
pub mod domain;
pub mod error;

pub use application::{children, kind_of, *};
pub use domain::*;
pub use error::ReflectError;

/// Derive macros (per-shape, ADR 0008): `Reflect` (base/Unknown), `ReflectStruct`,
/// `ReflectEnum`. Re-exported serde-style: the derive `Reflect` lives in the
/// macro namespace, the trait `Reflect` (via `domain::*`) in the type
/// namespace, so one `use dyn_rt_reflect::Reflect` brings in both.
pub use dyn_rt_reflect_derive::{Reflect, ReflectEnum, ReflectStruct};
