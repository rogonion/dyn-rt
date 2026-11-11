//! Data-driven suites for the regex PathTokenizer steps and segment extraction.
//! Vectors: tests/shared/json/path-split-*.json and path-extract-segment.json (ADR 0004).

use dyn_rt_json_domain::ports::PathTokenizer;
use dyn_rt_json_infra::RegexPathTokenizer;
use dyn_rt_test_support as support;

fn expected_strings(case: &serde_json::Value) -> Vec<String> {
    case["expected"]
        .as_array()
        .expect("expected must be an array")
        .iter()
        .map(|v| {
            v.as_str()
                .expect("expected entries must be strings")
                .to_string()
        })
        .collect()
}

fn check_split(cases_file: &str, op: &str) {
    let tokenizer = RegexPathTokenizer;
    for case in support::load_cases(cases_file) {
        let name = case["name"].as_str().unwrap_or("<unnamed>");
        let input = case["input"].as_str().unwrap();

        let parts: Vec<&str> = match op {
            "split_recursive_descent" => tokenizer.split_recursive_descent(input),
            "split_dot_notation" => tokenizer.split_dot_notation(input),
            "split_brackets" => tokenizer.split_brackets(input),
            other => panic!("unknown split op: {other}"),
        };
        let result: Vec<String> = parts.iter().map(|s| s.to_string()).collect();

        assert_eq!(
            result,
            expected_strings(&case),
            "case '{name}' failed:\n{}",
            serde_json::to_string_pretty(&case).unwrap()
        );
    }
}

#[test]
fn split_recursive_descent() {
    check_split(
        "json/path-split-recursive-descent.json",
        "split_recursive_descent",
    );
}

#[test]
fn split_dot_notation() {
    check_split("json/path-split-dot-notation.json", "split_dot_notation");
}

#[test]
fn split_brackets() {
    check_split("json/path-split-brackets.json", "split_brackets");
}

#[test]
fn extract_segment_cases() {
    let tokenizer = RegexPathTokenizer;
    for case in support::load_cases("json/path-extract-segment.json") {
        let name = case["name"].as_str().unwrap_or("<unnamed>");
        let input = case["input"].as_str().unwrap();
        let result = dyn_rt_json_application::extract_segment(&tokenizer, input);
        assert_eq!(
            support::segment_to_json(&result),
            case["expected"],
            "case '{name}' failed:\n{}",
            serde_json::to_string_pretty(&case).unwrap()
        );
    }
}
