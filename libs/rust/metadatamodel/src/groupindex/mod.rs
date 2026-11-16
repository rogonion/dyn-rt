//! `groupindex` subdomain: the reusable group-index machinery — instance
//! collections keyed by signature and the signature generator — extracted
//! from `unflattener` so other subdomains (row merging/dedup, database
//! upsert keying, ...) can use it on its own. Pure data structures over
//! `shared::Value` rows; no dependency on the metadata model itself.

pub mod domain;

pub use domain::*;
