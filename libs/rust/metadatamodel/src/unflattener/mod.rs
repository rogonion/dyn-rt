//! `unflattener` subdomain: convert a 2D table back into a nested
//! collection (port of go-metadatamodel/unflattener).

pub mod application;
pub mod domain;

pub use application::*;
pub use domain::*;
