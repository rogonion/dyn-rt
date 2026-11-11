//! dyn-rt json domain: the pure core of the `json` context.
//!
//! No external dependencies, no IO. Adapters (`dyn-rt-json-infra`) convert
//! between [`Value`] and concrete representations (e.g. `serde_json::Value`);
//! use cases (`dyn-rt-json-application`) operate on [`Value`] and paths only.

pub mod error;
pub mod path;
pub mod value;

pub use error::JsonError;
pub use path::{
    AccessStrategy, JsonPath, PathSegment, PathSelector, RecursiveDescentSegment,
    RecursiveDescentSegments, SliceSelector,
};
pub use value::{Number, Value};
