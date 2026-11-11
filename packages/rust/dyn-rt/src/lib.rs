//! dyn-rt — the dynamic runtime library (Rust port).
//!
//! One crate, one module per context; cargo features select what compiles so
//! dependents only build what they use (ADR 0003):
//!
//! - `json` (default): JSONPath-capable JSON toolkit.
//! - `metadatamodel`: metadata model on top of `json` (added in its phase).

#[cfg(feature = "json")]
pub mod json {
    //! JSON toolkit: value model, path parsing/resolution, object ops, schema.
    pub use dyn_rt_json_domain::*;
}
