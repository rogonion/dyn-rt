//! Shared fixtures moved to `dyn-rt-test-support::fixtures` (dev-only) so
//! other modules' suites (e.g. manipulation) can reuse the same shapes;
//! re-exported here for the existing `use common::*;` imports (ADR 0009).
#![allow(dead_code, unused_imports)]

pub use dyn_rt_test_support::fixtures::*;
