//! Data-driven suite for full path parse + Display round-trip (ADR 0004).
//! Case table built with `parse_case!` and the `json_path_*_segment!` /
//! `new_recursive_descent_segment(s)!` constructor macros, so the expected
//! AST reads like the path itself.

mod common;

use common::{ParseCase, parse_case};
use dyn_rt_json_path::domain::AccessStrategy;
use dyn_rt_json_path::{
    RegexPathTokenizer, json_path_index_selector_segment, json_path_key_selector_segment,
    json_path_root_segment, json_path_slice_selector_segment, json_path_union_selector_segment,
    json_path_wildcard_segment, new_recursive_descent_segment, new_recursive_descent_segments,
};
use dyn_rt_test_support::new_test_metadata;
use dyn_rt_test_support::{new_test_title, report_failures};

#[test]
fn path_parse() {
    let test_data = yield_parse_data();
    let mut failures: Vec<String> = Vec::new();

    for (test_index, case) in test_data.iter().enumerate() {
        let result = dyn_rt_json_path::parse_path(&RegexPathTokenizer, case.source);

        if result != case.expected {
            failures.push(format!(
                "{} || parse mismatch for path {} || expected {:?} ; got {:?}",
                new_test_title!(test_index + 1, case.meta.test_title),
                case.source,
                case.expected,
                result,
            ));
        }

        let rendered = result.to_string();
        if rendered != case.roundtrip {
            failures.push(format!(
                "{} || round-trip mismatch for path {} || expected {:?} ; got {:?}",
                new_test_title!(test_index + 1, case.meta.test_title),
                case.source,
                case.roundtrip,
                rendered,
            ));
        }
    }

    report_failures(failures, test_data.len());
}

fn yield_parse_data() -> Vec<ParseCase> {
    vec![
        parse_case!(
            "$.store.book[0].title",
            new_recursive_descent_segments!(new_recursive_descent_segment!(
                json_path_root_segment!(),
                json_path_key_selector_segment!("store"),
                json_path_key_selector_segment!("book"),
                json_path_index_selector_segment!(0),
                json_path_key_selector_segment!("title")
            )),
            roundtrip = "$.store.book[0].title",
            test_title = "standard dot and index path"
        ),
        parse_case!(
            "$.store..book[0].title",
            new_recursive_descent_segments!(
                new_recursive_descent_segment!(
                    json_path_root_segment!(),
                    json_path_key_selector_segment!("store")
                ),
                new_recursive_descent_segment!(
                    json_path_key_selector_segment!("book"),
                    json_path_index_selector_segment!(0),
                    json_path_key_selector_segment!("title")
                )
            ),
            roundtrip = "$.store..book[0].title",
            test_title = "basic path with recursive descent"
        ),
        parse_case!(
            "$..store..book..*",
            new_recursive_descent_segments!(
                new_recursive_descent_segment!(json_path_root_segment!()),
                new_recursive_descent_segment!(json_path_key_selector_segment!("store")),
                new_recursive_descent_segment!(json_path_key_selector_segment!("book")),
                new_recursive_descent_segment!(json_path_wildcard_segment!())
            ),
            roundtrip = "$..store..book..*",
            test_title = "multiple recursive descents ending in wildcard"
        ),
        parse_case!(
            "$['a..b']..['c..d']",
            new_recursive_descent_segments!(
                new_recursive_descent_segment!(
                    json_path_root_segment!(),
                    json_path_key_selector_segment!("a..b")
                ),
                new_recursive_descent_segment!(json_path_key_selector_segment!("c..d"))
            ),
            roundtrip = "$['a..b']..['c..d']",
            test_title = "string literals containing .. inside brackets"
        ),
        parse_case!(
            "$['report-data']..['total.sum..()&^']..store.bicycle['item-code']",
            new_recursive_descent_segments!(
                new_recursive_descent_segment!(
                    json_path_root_segment!(),
                    json_path_key_selector_segment!("report-data")
                ),
                new_recursive_descent_segment!(json_path_key_selector_segment!("total.sum..()&^")),
                new_recursive_descent_segment!(
                    json_path_key_selector_segment!("store"),
                    json_path_key_selector_segment!("bicycle"),
                    json_path_key_selector_segment!("item-code")
                )
            ),
            roundtrip = "$['report-data']..['total.sum..()&^']..store.bicycle['item-code']",
            test_title = "complex symbols with .. inside bracket quotes"
        ),
        parse_case!(
            "$['report-data']..['total.sum']..store.bicycle['item-code']",
            new_recursive_descent_segments!(
                new_recursive_descent_segment!(
                    json_path_root_segment!(),
                    json_path_key_selector_segment!("report-data")
                ),
                new_recursive_descent_segment!(json_path_key_selector_segment!("total.sum")),
                new_recursive_descent_segment!(
                    json_path_key_selector_segment!("store"),
                    json_path_key_selector_segment!("bicycle"),
                    json_path_key_selector_segment!("item-code")
                )
            ),
            roundtrip = "$['report-data']..['total.sum']..store.bicycle['item-code']",
            test_title = "multiple recursive descents with hyphenated keys"
        ),
        parse_case!(
            "$.transactions[1:5:2].amount",
            new_recursive_descent_segments!(new_recursive_descent_segment!(
                json_path_root_segment!(),
                json_path_key_selector_segment!("transactions"),
                json_path_slice_selector_segment!(1:5:2),
                json_path_key_selector_segment!("amount")
            )),
            roundtrip = "$.transactions[1:5:2].amount",
            test_title = "slice selector"
        ),
        parse_case!(
            "$.transactions[1:-5].amount",
            new_recursive_descent_segments!(new_recursive_descent_segment!(
                json_path_root_segment!(),
                json_path_key_selector_segment!("transactions"),
                json_path_slice_selector_segment!(1:-5),
                json_path_key_selector_segment!("amount")
            )),
            roundtrip = "$.transactions[1:-5].amount",
            test_title = "slice selector with negative end"
        ),
        parse_case!(
            "$.data.user.preferences['theme-settings',\"font-size\",-3]",
            new_recursive_descent_segments!(new_recursive_descent_segment!(
                json_path_root_segment!(),
                json_path_key_selector_segment!("data"),
                json_path_key_selector_segment!("user"),
                json_path_key_selector_segment!("preferences"),
                json_path_union_selector_segment!(
                    AccessStrategy::Mixed,
                    json_path_key_selector_segment!("theme-settings"),
                    json_path_key_selector_segment!("font-size"),
                    json_path_index_selector_segment!(-3)
                )
            )),
            roundtrip = "$.data.user.preferences['theme-settings','font-size',-3]",
            test_title = "union selector with mixed member types"
        ),
        parse_case!(
            "$[1,3,5]",
            new_recursive_descent_segments!(new_recursive_descent_segment!(
                json_path_root_segment!(),
                json_path_union_selector_segment!(
                    AccessStrategy::Positional,
                    json_path_index_selector_segment!(1),
                    json_path_index_selector_segment!(3),
                    json_path_index_selector_segment!(5)
                )
            )),
            roundtrip = "$[1,3,5]",
            test_title = "union of indices"
        ),
        parse_case!(
            "$..['1st_category'].name",
            new_recursive_descent_segments!(
                new_recursive_descent_segment!(json_path_root_segment!()),
                new_recursive_descent_segment!(
                    json_path_key_selector_segment!("1st_category"),
                    json_path_key_selector_segment!("name")
                )
            ),
            roundtrip = "$..['1st_category'].name",
            test_title = "recursive descent into bracketed numeric-leading key"
        ),
        parse_case!(
            "$.products['item-details..'].dimensions[0].width.dimensions[-2][3].width",
            new_recursive_descent_segments!(new_recursive_descent_segment!(
                json_path_root_segment!(),
                json_path_key_selector_segment!("products"),
                json_path_key_selector_segment!("item-details.."),
                json_path_key_selector_segment!("dimensions"),
                json_path_index_selector_segment!(0),
                json_path_key_selector_segment!("width"),
                json_path_key_selector_segment!("dimensions"),
                json_path_index_selector_segment!(-2),
                json_path_index_selector_segment!(3),
                json_path_key_selector_segment!("width")
            )),
            roundtrip = "$.products['item-details..'].dimensions[0].width.dimensions[-2][3].width",
            test_title = "dots inside property key plus chained array indices"
        ),
        parse_case!(
            "$['user info']['address.wind'][1].street",
            new_recursive_descent_segments!(new_recursive_descent_segment!(
                json_path_root_segment!(),
                json_path_key_selector_segment!("user info"),
                json_path_key_selector_segment!("address.wind"),
                json_path_index_selector_segment!(1),
                json_path_key_selector_segment!("street")
            )),
            roundtrip = "$['user info']['address.wind'][1].street",
            test_title = "spaces and dots in quoted keys"
        ),
        parse_case!(
            "$.data[*].price",
            new_recursive_descent_segments!(new_recursive_descent_segment!(
                json_path_root_segment!(),
                json_path_key_selector_segment!("data"),
                json_path_wildcard_segment!(),
                json_path_key_selector_segment!("price")
            )),
            roundtrip = "$.data.*.price",
            test_title = "wildcard member"
        ),
        parse_case!(
            "$.store.bicycle['item-code']",
            new_recursive_descent_segments!(new_recursive_descent_segment!(
                json_path_root_segment!(),
                json_path_key_selector_segment!("store"),
                json_path_key_selector_segment!("bicycle"),
                json_path_key_selector_segment!("item-code")
            )),
            roundtrip = "$.store.bicycle['item-code']",
            test_title = "bracketed property key"
        ),
        parse_case!(
            "$..author",
            new_recursive_descent_segments!(
                new_recursive_descent_segment!(json_path_root_segment!()),
                new_recursive_descent_segment!(json_path_key_selector_segment!("author"))
            ),
            roundtrip = "$..author",
            test_title = "single recursive descent"
        ),
        parse_case!(
            "$['store\\'s..item']..id",
            new_recursive_descent_segments!(
                new_recursive_descent_segment!(
                    json_path_root_segment!(),
                    json_path_key_selector_segment!("store\\'s..item")
                ),
                new_recursive_descent_segment!(json_path_key_selector_segment!("id"))
            ),
            roundtrip = "$['store\\'s..item']..id",
            test_title = "escaped single quote inside key containing .."
        ),
        parse_case!(
            "$[\"user\\\"name..data\"]..title",
            new_recursive_descent_segments!(
                new_recursive_descent_segment!(
                    json_path_root_segment!(),
                    json_path_key_selector_segment!("user\\\"name..data")
                ),
                new_recursive_descent_segment!(json_path_key_selector_segment!("title"))
            ),
            roundtrip = "$['user\\\"name..data']..title",
            test_title = "escaped double quote inside key containing .."
        ),
        parse_case!(
            "$['']..author",
            new_recursive_descent_segments!(
                new_recursive_descent_segment!(
                    json_path_root_segment!(),
                    json_path_key_selector_segment!("")
                ),
                new_recursive_descent_segment!(json_path_key_selector_segment!("author"))
            ),
            roundtrip = "$['']..author",
            test_title = "empty bracketed string key"
        ),
        parse_case!(
            "$['title_\"main\"..part']..id",
            new_recursive_descent_segments!(
                new_recursive_descent_segment!(
                    json_path_root_segment!(),
                    json_path_key_selector_segment!("title_\"main\"..part")
                ),
                new_recursive_descent_segment!(json_path_key_selector_segment!("id"))
            ),
            roundtrip = "$['title_\"main\"..part']..id",
            test_title = "double quotes inside single-quoted key with .."
        ),
        parse_case!(
            "$['report-data']..['total.sum']",
            new_recursive_descent_segments!(
                new_recursive_descent_segment!(
                    json_path_root_segment!(),
                    json_path_key_selector_segment!("report-data")
                ),
                new_recursive_descent_segment!(json_path_key_selector_segment!("total.sum"))
            ),
            roundtrip = "$['report-data']..['total.sum']",
            test_title = "two recursive descent groups with hyphenated keys (go/js variant)"
        ),
        parse_case!(
            "$.transactions[1:5:2].*",
            new_recursive_descent_segments!(new_recursive_descent_segment!(
                json_path_root_segment!(),
                json_path_key_selector_segment!("transactions"),
                json_path_slice_selector_segment!(1:5:2),
                json_path_wildcard_segment!()
            )),
            roundtrip = "$.transactions[1:5:2].*",
            test_title = "slice followed by wildcard (js variant)"
        ),
        parse_case!(
            "$.data.user.preferences['theme-settings','font-size']",
            new_recursive_descent_segments!(new_recursive_descent_segment!(
                json_path_root_segment!(),
                json_path_key_selector_segment!("data"),
                json_path_key_selector_segment!("user"),
                json_path_key_selector_segment!("preferences"),
                json_path_union_selector_segment!(
                    AccessStrategy::Named,
                    json_path_key_selector_segment!("theme-settings"),
                    json_path_key_selector_segment!("font-size")
                )
            )),
            roundtrip = "$.data.user.preferences['theme-settings','font-size']",
            test_title = "union of two named keys (go/js variant)"
        ),
        parse_case!(
            "$.products['item-details'].dimensions[0].width",
            new_recursive_descent_segments!(new_recursive_descent_segment!(
                json_path_root_segment!(),
                json_path_key_selector_segment!("products"),
                json_path_key_selector_segment!("item-details"),
                json_path_key_selector_segment!("dimensions"),
                json_path_index_selector_segment!(0),
                json_path_key_selector_segment!("width")
            )),
            roundtrip = "$.products['item-details'].dimensions[0].width",
            test_title = "hyphenated key without inner dots (go/js variant)"
        ),
    ]
}
