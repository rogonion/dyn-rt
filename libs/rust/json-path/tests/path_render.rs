//! Data-driven round-trip suite for path rendering: builds the AST from each
//! case's `segments` and asserts it renders back to `roundtrip`.
//! Vectors: tests/shared/json/path-parse.json (ADR 0004).

use dyn_rt_test_support as support;

#[test]
fn path_render_roundtrip() {
    for case in support::load_cases("json/path-parse.json") {
        let name = case["name"].as_str().unwrap_or("<unnamed>");
        let segments = support::segments_from_json(&case["segments"]);
        let rendered = segments.to_string();
        assert_eq!(
            rendered,
            case["roundtrip"].as_str().unwrap(),
            "case '{name}' failed:\n{}",
            serde_json::to_string_pretty(&case).unwrap()
        );
    }
}
