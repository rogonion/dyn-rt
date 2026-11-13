//! `for_each`: invoke a callback for every value found at a JSONPath in a
//! reflected source (port of go-json's `object.ForEach`).
//!
//! The traversal is the same one `get` uses, so every selector form get
//! supports (recursive descent, wildcard, unions incl. negative/slice
//! members, slices) applies here too. The callback returns `false` to stop
//! the iteration early (go's `IfValueFoundInObject` bool).

use dyn_rt_json_path::RecursiveDescentSegments;
use dyn_rt_reflect::Reflect;

use super::get;
use crate::error::ManipulationError;

/// Calls `callback` for each value in `source` matched by `segments`, in
/// traversal order. Returning `false` from the callback stops early.
pub fn for_each(
    source: &dyn Reflect,
    segments: &RecursiveDescentSegments,
    callback: &mut dyn FnMut(&dyn Reflect) -> bool,
) -> Result<(), ManipulationError> {
    for value in get(source, segments)? {
        if !callback(value) {
            break;
        }
    }
    Ok(())
}
