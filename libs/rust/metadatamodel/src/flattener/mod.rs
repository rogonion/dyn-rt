//! `flattener` subdomain: convert nested objects/collections into a 2D
//! table (port of go-metadatamodel/flattener).

pub mod application;
pub mod domain;

pub use application::*;
pub use domain::*;
