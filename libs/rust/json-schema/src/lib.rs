//! dyn-rt json-schema module.
//!
//! A JSON-like schema tree describing the expected structure of data, keyed
//! off the reflect module's kinds. Layers per ADR 0002/0007:
//! `domain` (schema model, std + reflect only), `application` (validation
//! and schema-path resolution), `adapters` (format deserialization — the
//! only place serde may be imported).

pub mod adapters;
pub mod application;
pub mod domain;
pub mod error;

pub use application::*;
pub use domain::*;
pub use error::SchemaError;
