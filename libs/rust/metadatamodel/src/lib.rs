//! dyn-rt metadatamodel module — the metadata model on top of the json
//! modules (ADR 0007: one submodule per subdomain under `src/`, each with
//! its own domain/application/adapters).
//!
//! Subdomains (port order): `core`, `fieldcolumns`, `database`, `filter`,
//! `iter`, `flattener`, `unflattener`.
//!
//! The metadata model is a JSON-like `shared::Value` tree, not a concrete
//! struct (extensibility), and field/group property keys are namespaced
//! `<section>:<key>`.

pub mod core;
pub mod error;
pub mod fieldcolumns;

pub use error::MetadatamodelError;
