//! `filter` subdomain: filter source data (whose structure is defined by a
//! metadata model) through query conditions — logical operators and
//! per-field conditions (port of go-metadatamodel/filter).

pub mod application;
pub mod conditions;
pub mod domain;

pub use application::*;
pub use conditions::*;
pub use domain::*;
