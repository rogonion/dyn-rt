//! dyn-rt shared substrate: the pure JSON value model (`Value`, `Number`) and
//! the json error vocabulary (`JsonError`) used by the json-* modules.
//!
//! std only, no IO. A shape lands here only when 2+ modules need it
//! (ADR 0007).

pub mod error;
pub mod value;

pub use error::JsonError;
pub use value::{Number, Value};
