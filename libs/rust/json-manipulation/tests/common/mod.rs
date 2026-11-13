//! Shared helpers for the json-manipulation suites (ADR 0009): the shared
//! fixtures from dyn-rt-test-support plus result renderers.
#![allow(dead_code, unused_imports)]

use std::any::TypeId;

use dyn_rt_reflect::Reflect;
use dyn_rt_shared::Value;
pub use dyn_rt_test_support::fixtures::*;

/// Renders a reflected result for comparison: `Value` via Debug, anything else
/// via its scalar display form (strings quoted).
pub fn render(r: &dyn Reflect) -> String {
    if let Some(v) = r.reflect_as_any().downcast_ref::<Value>() {
        return format!("{v:?}");
    }
    render_scalar(r)
}

/// Renders a scalar-ish reflected value: bools/numbers bare, strings quoted.
pub fn render_scalar(r: &dyn Reflect) -> String {
    if let Some(b) = r.reflect_scalar_bool() {
        return format!("{b}");
    }
    if let Some(i) = r.reflect_scalar_i64() {
        return format!("{i}");
    }
    if let Some(f) = r.reflect_scalar_f64() {
        return format!("{f}");
    }
    if let Some(s) = r.reflect_scalar_string() {
        if r.reflect_as_any().type_id() == TypeId::of::<String>() {
            return format!("{s:?}");
        }
        return s;
    }
    r.reflect_type_name().to_string()
}

/// Renders via the concrete type's `Debug` when it downcasts, else falls back
/// to [`render`]. Use as `render_debug::<UserProfile>`.
pub fn render_debug<T: std::fmt::Debug + 'static>(r: &dyn Reflect) -> String {
    match r.reflect_as_any().downcast_ref::<T>() {
        Some(v) => format!("{v:?}"),
        None => render(r),
    }
}
