//! Dev-only test harness shared by dyn-rt test suites (ADR 0004), ported
//! from dyn-rt-rs-old-3's `shared` crate: case tables are plain in-language
//! structs built with constructor macros, each suite is one `#[test]` looping
//! over a `yield_test_data()`-style table, and failures are collected and
//! reported once with the case title and expected-vs-got. This crate must
//! only ever be referenced from `dev-dependencies` — never from shipped code.

mod harness;

pub use harness::*;
