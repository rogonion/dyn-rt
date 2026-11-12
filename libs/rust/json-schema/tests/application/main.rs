//! Application-layer test suites for json-schema (ADR 0009): one file per
//! src/application/ file.

#[path = "../common/mod.rs"]
mod common;

mod path;
mod validation;
