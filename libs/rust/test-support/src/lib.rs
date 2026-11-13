//! Dev-only test harness shared by dyn-rt test suites (ADR 0004), ported
//! from dyn-rt-rs-old-3's `shared` crate: case tables are plain in-language
//! structs (each carrying a `test_title` field) built with constructor
//! macros, each suite is one `#[test]` looping over a `yield_test_data()`-style
//! table and recording outcomes in a `TestReport`, which panics once with the
//! case title and expected-vs-got on any failure and otherwise logs the passed
//! coverage. This crate must only ever be referenced from `dev-dependencies`
//! — never from shipped code.

mod harness;

pub mod fixtures;

pub use harness::*;
