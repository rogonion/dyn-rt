//! dyn-rt — the dynamic runtime library (Rust port).
//!
//! One crate, one module per context/feature so dependents only build what
//! they use (ADR 0003). Module layout per ADR 0007:
//!
//! - `json::path` (feature `json-path`, in default `json`): JSONPath parsing.
//! - `json::manipulation`, `json::schema`: added per module.
//! - `reflect` (feature `reflect`, in default): general-purpose runtime
//!   reflection — usable independently of json.
//! - `metadatamodel`: metadata model on top of `json` (added in its phase).

#[cfg(feature = "reflect")]
pub mod reflect {
    //! General-purpose runtime reflection (the go-`reflect` analogue).
    pub use dyn_rt_reflect::*;
}

#[cfg(feature = "json-path")]
pub mod json {
    //! JSON toolkit modules. The shared value model and error vocabulary are
    //! re-exported here for convenience.
    pub use dyn_rt_shared::{JsonError, Number, Value};

    pub mod path {
        //! JSONPath: AST (domain), parse use cases (application), regex
        //! tokenizer (adapters).
        pub use dyn_rt_json_path::adapters::RegexPathTokenizer;
        pub use dyn_rt_json_path::application::{extract_segment, parse_path};
        pub use dyn_rt_json_path::domain::*;
        // AST constructor macros (for writing path tables, e.g. in tests)
        pub use dyn_rt_json_path::{
            json_path_index_selector_segment, json_path_key_selector_segment,
            json_path_root_segment, json_path_slice_selector_segment,
            json_path_union_selector_segment, json_path_wildcard_segment,
            new_recursive_descent_segment, new_recursive_descent_segments,
        };

        /// Parse a JSONPath string into its deconstructed AST, using the
        /// default regex tokenizer.
        ///
        /// Example: `$.store.book[0].title`
        pub fn parse(path: &str) -> RecursiveDescentSegments {
            parse_path(&RegexPathTokenizer, path)
        }
    }

    #[cfg(feature = "json-schema")]
    pub mod schema {
        //! JSON-like schema: model (domain), validation + schema-path
        //! (application), format deserialization adapters.
        pub use dyn_rt_json_schema::adapters::*;
        pub use dyn_rt_json_schema::application::*;
        pub use dyn_rt_json_schema::domain::*;
        pub use dyn_rt_json_schema::error::SchemaError;
    }

    #[cfg(feature = "json-manipulation")]
    pub mod manipulation {
        //! JSON manipulation over reflected data: get / set / delete at a
        //! JSONPath (domain cursor, application use cases, string-path
        //! adapters).
        pub use dyn_rt_json_manipulation::adapters::*;
        pub use dyn_rt_json_manipulation::application::*;
        pub use dyn_rt_json_manipulation::domain::*;
        pub use dyn_rt_json_manipulation::error::ManipulationError;
    }
}

#[cfg(feature = "metadatamodel")]
pub mod metadatamodel {
    //! Metadata model on top of the json modules: one submodule per
    //! subdomain (core, fieldcolumns, database, filter, iter, flattener,
    //! unflattener), each with domain/application/adapters.
    pub use dyn_rt_metadatamodel::core;
    pub use dyn_rt_metadatamodel::database;
    pub use dyn_rt_metadatamodel::error::MetadatamodelError;
    pub use dyn_rt_metadatamodel::fieldcolumns;
}
