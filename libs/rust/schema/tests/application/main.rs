//! Application-layer test suites for schema (ADR 0009): one file per
//! src/application/ file.

#[path = "../common/mod.rs"]
mod common;

mod conversion;
mod path;
mod validation;
