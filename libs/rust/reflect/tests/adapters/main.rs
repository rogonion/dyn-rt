//! Adapter-layer suites for the reflect module (ADR 0004/0007): one submodule
//! per `src/adapters/` file, so the test tree mirrors the adapter folder.

#[path = "../common/mod.rs"]
mod common;

mod enum_shape;
mod indirection;
mod map;
mod scalar;
mod sequence;
mod unknown;
