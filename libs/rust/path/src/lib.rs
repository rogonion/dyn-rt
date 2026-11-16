//! dyn-rt path module — one crate, hexagon inside (ADR 0007):
//!
//! - `domain`: the JSONPath AST and the [`domain::PathTokenizer`] port (std only)
//! - `application`: the parse use cases (imports domain only)
//! - `adapters`: the regex tokenizer — the only place `regex` is imported

mod macros;

pub mod adapters;
pub mod application;
pub mod domain;

pub use adapters::RegexPathTokenizer;
pub use application::{extract_segment, parse_path};
pub use domain::*;
