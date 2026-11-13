//! `core` subdomain: the metadata model vocabulary (domain) and the
//! property/path-template use cases (application). Port of go-metadatamodel's
//! `core` package, with `<section>:<key>` property names and the model as a
//! JSON-like `Value` tree.

pub mod application;
pub mod domain;

pub use application::*;
pub use domain::*;
