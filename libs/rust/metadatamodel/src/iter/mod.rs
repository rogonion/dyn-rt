//! `iter` subdomain: recursive walks over the metadata model's
//! fields/groups — for_each / filter / map (port of go-metadatamodel/iter).

pub mod application;
pub mod domain;

pub use application::*;
pub use domain::*;
