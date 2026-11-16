//! dyn-rt — the dynamic runtime library (Rust port).
//!
//! dyn-rt is a dynamic layer over a language's own data types (built-ins and
//! user-defined structs/classes), explored and manipulated at any tree depth
//! via JSONPath-style paths, reflection, and schemas. `json` names only the
//! JSON data model (ADR 0010).
//!
//! One crate, one module per context/feature so dependents only build what
//! they use (ADR 0003). Module layout per ADR 0007:
//!
//! - `json` (feature `json`, in default): the JSON value model —
//!   `Value` / `Number` / `JsonError`.
//! - `path` (feature `path`, in default): JSONPath parsing/resolution over
//!   any reflected tree.
//! - `schema` (feature `schema`, in default): schemas over reflected data.
//! - `manipulation` (feature `manipulation`, in default): `DynObject` +
//!   get/set/delete/for_each at a JSONPath.
//! - `reflect` (feature `reflect`, in default): general-purpose runtime
//!   reflection — usable independently of the rest.
//! - `metadatamodel`: metadata model on top of the dynamic layer.

#[cfg(feature = "json")]
pub mod json {
    //! The JSON data model: the pure `Value`/`Number` types and the `JsonError`
    //! vocabulary. This is the only place `json` appears as a module name
    //! (ADR 0010) — everything else is the dynamic layer over native types.
    pub use dyn_rt_shared::{JsonError, Number, Value};
}

#[cfg(feature = "reflect")]
pub mod reflect {
    //! General-purpose runtime reflection (the go-`reflect` analogue).
    pub use dyn_rt_reflect::*;
}

#[cfg(feature = "path")]
pub mod path {
    //! JSONPath over any reflected tree: AST (domain), parse use cases
    //! (application), regex tokenizer (adapters).
    pub use dyn_rt_path::adapters::RegexPathTokenizer;
    pub use dyn_rt_path::application::{extract_segment, parse_path};
    pub use dyn_rt_path::domain::*;
    // AST constructor macros (for writing path tables, e.g. in tests)
    pub use dyn_rt_path::{
        json_path_index_selector_segment, json_path_key_selector_segment, json_path_root_segment,
        json_path_slice_selector_segment, json_path_union_selector_segment,
        json_path_wildcard_segment, new_recursive_descent_segment, new_recursive_descent_segments,
    };

    /// Parse a JSONPath string into its deconstructed AST, using the
    /// default regex tokenizer.
    ///
    /// Example: `$.store.book[0].title`
    pub fn parse(path: &str) -> RecursiveDescentSegments {
        parse_path(&RegexPathTokenizer, path)
    }
}

#[cfg(feature = "schema")]
pub mod schema {
    //! Schemas over reflected data: model (domain), validation + schema-path
    //! + conversion (application), format deserialization adapters.
    pub use dyn_rt_schema::adapters::*;
    pub use dyn_rt_schema::application::*;
    pub use dyn_rt_schema::domain::*;
    pub use dyn_rt_schema::error::SchemaError;
}

#[cfg(feature = "manipulation")]
pub mod manipulation {
    //! Dynamic manipulation of reflected data: `DynObject` (the entrypoint
    //! wrapper) plus get / set / delete / for_each at a JSONPath (domain
    //! cursor, application use cases, string-path adapters).
    pub use dyn_rt_manipulation::adapters::*;
    pub use dyn_rt_manipulation::application::*;
    pub use dyn_rt_manipulation::domain::*;
    pub use dyn_rt_manipulation::error::ManipulationError;
}

#[cfg(feature = "metadatamodel")]
pub mod metadatamodel {
    //! Metadata model on top of the dynamic layer: one submodule per
    //! subdomain (core, fieldcolumns, database, filter, iter, flattener,
    //! unflattener, groupindex), each with domain/application/adapters.
    pub use dyn_rt_metadatamodel::core;
    pub use dyn_rt_metadatamodel::database;
    pub use dyn_rt_metadatamodel::error::MetadatamodelError;
    pub use dyn_rt_metadatamodel::fieldcolumns;
    pub use dyn_rt_metadatamodel::filter;
    pub use dyn_rt_metadatamodel::flattener;
    pub use dyn_rt_metadatamodel::groupindex;
    pub use dyn_rt_metadatamodel::iter;
    pub use dyn_rt_metadatamodel::unflattener;
}
