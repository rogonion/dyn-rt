//! Application use cases for json-schema, one file per concern:
//! `validation` (data against schema), `path` (schema lookup by JSONPath),
//! `conversion` (data into the schema-dictated shape — stage 2).

pub mod conversion;
pub mod path;
pub mod validation;

pub use conversion::*;
pub use path::*;
pub use validation::*;
