//! Adapters: string-path entry points. Parsing uses json-path's regex
//! tokenizer (the only place that crate's adapter is driven from here), then
//! delegates to the application use cases.

use dyn_rt_json_path::{RecursiveDescentSegments, RegexPathTokenizer, parse_path};
use dyn_rt_json_schema::Schema;
use dyn_rt_reflect::Reflect;

use crate::application::{delete, get, set};
use crate::error::ManipulationError;

/// Parses `path` and gets the value(s) at it from `source`.
pub fn get_str<'a>(
    source: &'a dyn Reflect,
    path: &str,
) -> Result<Vec<&'a dyn Reflect>, ManipulationError> {
    let segments: RecursiveDescentSegments = parse_path(&RegexPathTokenizer, path);
    get(source, &segments)
}

/// Parses `path` and sets `value` at it in `source` (see [`set`]).
pub fn set_str(
    source: &mut dyn Reflect,
    path: &str,
    value: &dyn Reflect,
    schema: Option<&Schema>,
) -> Result<usize, ManipulationError> {
    let segments: RecursiveDescentSegments = parse_path(&RegexPathTokenizer, path);
    set(source, &segments, value, schema)
}

/// Parses `path` and deletes the value(s) at it from `source` (see [`delete`]).
pub fn delete_str(source: &mut dyn Reflect, path: &str) -> Result<usize, ManipulationError> {
    let segments: RecursiveDescentSegments = parse_path(&RegexPathTokenizer, path);
    delete(source, &segments)
}
