//! Data-driven suite for extract_segment (ADR 0004). Case table built with
//! `extract_case!` and the `json_path_*_segment!` constructor macros.

mod common;

use common::{ExtractCase, extract_case};
use dyn_rt_json_path::domain::AccessStrategy;
use dyn_rt_json_path::{RegexPathTokenizer, extract_segment};
use dyn_rt_json_path::{
    json_path_index_selector_segment, json_path_key_selector_segment, json_path_root_segment,
    json_path_slice_selector_segment, json_path_union_selector_segment, json_path_wildcard_segment,
};
use dyn_rt_test_support::{new_test_metadata, new_test_title, report_failures};

#[test]
fn extract_segment_cases() {
    let test_data = yield_extract_segment_data();
    let tokenizer = RegexPathTokenizer;
    let mut failures: Vec<String> = Vec::new();

    for (test_index, case) in test_data.iter().enumerate() {
        let result = extract_segment(&tokenizer, case.source);
        if result != case.expected {
            failures.push(format!(
                "{} || expected {:?} ; got {:?}",
                new_test_title!(test_index + 1, case.meta.test_title),
                case.expected,
                result,
            ));
        }
    }

    report_failures(failures, test_data.len());
}

fn yield_extract_segment_data() -> Vec<ExtractCase> {
    vec![
        extract_case!(
            "$",
            json_path_root_segment!(),
            test_title = "standalone root selector"
        ),
        extract_case!(
            "store",
            json_path_key_selector_segment!("store"),
            test_title = "standalone property name"
        ),
        extract_case!(
            "*",
            json_path_wildcard_segment!(),
            test_title = "standalone wildcard selector"
        ),
        extract_case!(
            "'a..b'",
            json_path_key_selector_segment!("a..b"),
            test_title = "quoted bracket key with inner dots"
        ),
        extract_case!(
            "'report-data'",
            json_path_key_selector_segment!("report-data"),
            test_title = "hyphenated quoted key"
        ),
        extract_case!(
            "'total.sum..()&^'",
            json_path_key_selector_segment!("total.sum..()&^"),
            test_title = "complex symbols and dots inside single quotes"
        ),
        extract_case!(
            "1:5:2",
            json_path_slice_selector_segment!(1:5:2),
            test_title = "slice start:end:step"
        ),
        extract_case!(
            "1:5",
            json_path_slice_selector_segment!(1:5),
            test_title = "slice start:end"
        ),
        extract_case!(
            "::-2",
            json_path_slice_selector_segment!(::-2),
            test_title = "slice step only"
        ),
        extract_case!(
            ":4:-2",
            json_path_slice_selector_segment!(:4:-2),
            test_title = "slice end:step"
        ),
        extract_case!(
            "'theme-settings',\"font-size\",-3, -1::2",
            json_path_union_selector_segment!(
                AccessStrategy::Mixed,
                json_path_key_selector_segment!("theme-settings"),
                json_path_key_selector_segment!("font-size"),
                json_path_index_selector_segment!(-3),
                json_path_slice_selector_segment!(-1::2)
            ),
            test_title = "mixed union selector with key, key, index, slice members"
        ),
        extract_case!(
            "'user info'",
            json_path_key_selector_segment!("user info"),
            test_title = "quoted key with space"
        ),
        extract_case!(
            "'store\\'s..item'",
            json_path_key_selector_segment!("store\\'s..item"),
            test_title = "escaped quote bracket key"
        ),
        extract_case!(
            "\"user\\\"name..data\"",
            json_path_key_selector_segment!("user\\\"name..data"),
            test_title = "escaped double quote bracket key"
        ),
        extract_case!(
            "''",
            json_path_key_selector_segment!(""),
            test_title = "empty string bracket key"
        ),
        extract_case!(
            "'title_\"main\"..part'",
            json_path_key_selector_segment!("title_\"main\"..part"),
            test_title = "mixed quote bracket key"
        ),
    ]
}
