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

        /// Parse a JSONPath string into its deconstructed AST, using the
        /// default regex tokenizer.
        ///
        /// Example: `$.store.book[0].title`
        pub fn parse(path: &str) -> RecursiveDescentSegments {
            parse_path(&RegexPathTokenizer, path)
        }
    }
}
