//! `fieldcolumns` subdomain: the metadata model's fields as columns of a
//! flat 2D table (port of go-metadatamodel/fieldcolumns).

pub mod application;
pub mod domain;

pub use application::*;
pub use domain::*;
