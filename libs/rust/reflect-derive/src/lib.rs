//! Derive macros for `dyn-rt-reflect` (ADR 0008). Per-shape, each derive emits
//! the base `Reflect` impl as well, so one derive per type:
//!
//! - `#[derive(Reflect)]` — base trait only, routes to `Unknown`; requires
//!   `Clone` (clone/apply are clone-based). Expansion in `base.rs`.
//! - `#[derive(ReflectStruct)]` — structs (non-generic for now); structural
//!   clone/apply field-by-field, so fields must implement `Reflect`. Expansion
//!   in `struct_derive.rs`.
//! - `#[derive(ReflectEnum)]` — enums with unit or single-payload tuple
//!   variants; structural clone/apply per payload. Expansion in
//!   `enum_derive.rs`.
//!
//! Attributes (all under `#[reflect(...)]`):
//! - type level: `crate = "path"` (reflect crate path, default `dyn_rt_reflect`),
//!   `name = "..."` (override reflect_type_name), `new` (emit ReflectNew via Default)
//! - struct fields: `rename = "..."`, `skip` (skipped fields clone via `Clone`)
//! - enum variants: `rename = "..."`
//!
//! Attribute parsing lives in `attrs.rs`; shared base-impl generation in
//! `base.rs`. Proc-macro entry points must stay at the crate root, so they are
//! thin wrappers below.

mod attrs;
mod base;
mod enum_derive;
mod struct_derive;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(Reflect, attributes(reflect))]
pub fn derive_reflect(input: TokenStream) -> TokenStream {
    base::expand_reflect(parse_macro_input!(input as DeriveInput))
}

#[proc_macro_derive(ReflectStruct, attributes(reflect))]
pub fn derive_reflect_struct(input: TokenStream) -> TokenStream {
    struct_derive::expand(parse_macro_input!(input as DeriveInput))
}

#[proc_macro_derive(ReflectEnum, attributes(reflect))]
pub fn derive_reflect_enum(input: TokenStream) -> TokenStream {
    enum_derive::expand(parse_macro_input!(input as DeriveInput))
}
