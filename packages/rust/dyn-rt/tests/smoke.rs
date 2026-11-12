//! Inline smoke test for the facade re-exports (ADR 0004 tier 3: this is not
//! data — it just proves the `dyn-rt` crate wiring exposes the module APIs;
//! the data-driven suites live in each module crate, e.g.
//! `libs/rust/json-path/tests/`).

use dyn_rt::json::path::{
    self, json_path_index_selector_segment, json_path_key_selector_segment, json_path_root_segment,
    new_recursive_descent_segment, new_recursive_descent_segments,
};
use dyn_rt::reflect::{self, Reflect};

#[test]
fn facade_exposes_json_path_and_reflect() {
    let segments = path::parse("$.store.book[0].title");
    assert_eq!(segments.to_string(), "$.store.book[0].title",);
    assert_eq!(
        segments,
        new_recursive_descent_segments!(new_recursive_descent_segment!(
            json_path_root_segment!(),
            json_path_key_selector_segment!("store"),
            json_path_key_selector_segment!("book"),
            json_path_index_selector_segment!(0),
            json_path_key_selector_segment!("title"),
        ))
    );

    let value = dyn_rt::json::Value::from(42i64);
    let reflected: &dyn Reflect = &value;
    assert_eq!(reflect::kind_of(reflected), reflect::ReflectKind::Scalar);
    assert_eq!(reflected.reflect_type_name(), "number");
}
