//! `database` subdomain: metadata model fields as columns of a relational
//! database table collection (port of go-metadatamodel/database).

pub mod application;
pub mod domain;

pub use application::*;
pub use domain::*;
