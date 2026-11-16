//! Adapter-layer suites for path (ADR 0009): one submodule per
//! `src/adapters/` file, so the test tree mirrors the adapter folder.

#[path = "../common/mod.rs"]
mod common;

mod regex_tokenizer;
