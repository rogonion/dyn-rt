//! Data-driven full-parse + round-trip suites for the json context, wired
//! through the public facade (regex tokenizer + parse orchestration).
//! Vectors: tests/shared/json/path-parse.json (ADR 0004).

use dyn_rt_test_support as support;

#[test]
fn path_parse() {
    for case in support::load_cases("json/path-parse.json") {
        let name = case["name"].as_str().unwrap_or("<unnamed>");
        let path = case["path"].as_str().unwrap();

        let result = dyn_rt::json::path::parse(path);

        assert_eq!(
            support::segments_to_json(&result),
            case["segments"],
            "case '{name}' parse mismatch for path {path}:\n{}",
            serde_json::to_string_pretty(&case).unwrap()
        );

        assert_eq!(
            result.to_string(),
            case["roundtrip"].as_str().unwrap(),
            "case '{name}' round-trip mismatch for path {path}"
        );
    }
}
