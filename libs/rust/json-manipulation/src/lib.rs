//! dyn-rt json-manipulation module — get/set/delete over reflected data via
//! JSONPath (port of go-json's `object` package onto dyn-rt's reflect).
//!
//! Layers per ADR 0002/0007:
//! - `domain`: the path cursor used while walking a parsed JSONPath
//! - `application`: the `get` / `set` / `delete` use cases over
//!   `&dyn Reflect` / `&mut dyn Reflect`, taking a parsed
//!   [`dyn_rt_json_path::RecursiveDescentSegments`], plus [`DynObject`] — the
//!   stateful entrypoint wrapper (borrowed or owned source, optional schema,
//!   JSONPath accepted as text or parsed segments)
//! - `adapters`: string-path entry points (parse via json-path's regex
//!   tokenizer, then delegate to application)
//!
//! `set` uses json-schema's conversion (and schema-at-path) to coerce the
//! value being set into the destination type and to create intermediate
//! structures when a schema is provided.

pub mod adapters;
pub mod application;
pub mod domain;
pub mod error;

pub use application::*;
pub use domain::*;
pub use error::ManipulationError;
