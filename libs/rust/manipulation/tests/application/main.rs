//! Application-layer test suites for manipulation (ADR 0009): one file
//! per src/application/ use case.

#[path = "../common/mod.rs"]
mod common;

mod are_equal;
mod delete;
mod for_each;
mod get;
mod object;
mod set;
