//! Application-layer suites for json-path (ADR 0009): the parse_path and
//! extract_segment use cases.

mod common;

use common::{ExtractCase, ParseCase, extract_case, parse_case};
use dyn_rt_json_path::domain::AccessStrategy;
use dyn_rt_json_path::{
    RegexPathTokenizer, extract_segment, json_path_index_selector_segment,
    json_path_key_selector_segment, json_path_root_segment, json_path_slice_selector_segment,
    json_path_union_selector_segment, json_path_wildcard_segment, new_recursive_descent_segment,
    new_recursive_descent_segments, parse_path,
};
use dyn_rt_test_support::{TestReport, new_test_title};

#[test]
fn parse_path_cases() {
    let test_data = yield_parse_data();
    let tokenizer = RegexPathTokenizer;
    let mut report = TestReport::new(
        "parse_path_cases",
        "Application-layer suites for json-path (ADR 0009): the parse_path and",
    );

    for case in &test_data {
        let title = new_test_title!(case.test_title);
        report.begin_case(title.clone());
        let result = parse_path(&tokenizer, case.source);
        if result != case.expected {
            report.fail_case(format!(
                "parse mismatch for path {} || expected {:?} ; got {:?}",
                case.source, case.expected, result,
            ));
        }
    }

    report.finish();
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
            test_title = "bracketed property key"
        ),
        parse_case!(
            "$..author",
            new_recursive_descent_segments!(
                new_recursive_descent_segment!(json_path_root_segment!()),
                new_recursive_descent_segment!(json_path_key_selector_segment!("author"))
            ),
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
            test_title = "union of two named keys (go/js variant)"
        ),
        parse_case!(
            "$.User['ID', 'Name']",
            new_recursive_descent_segments!(new_recursive_descent_segment!(
                json_path_root_segment!(),
                json_path_key_selector_segment!("User"),
                json_path_union_selector_segment!(
                    AccessStrategy::Named,
                    json_path_key_selector_segment!("ID"),
                    json_path_key_selector_segment!("Name")
                )
            )),
            test_title = "union members after a comma+space are still unquoted"
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
            test_title = "hyphenated key without inner dots (go/js variant)"
        ),
    ]
}

#[test]
fn extract_segment_cases() {
    let test_data = yield_extract_data();
    let tokenizer = RegexPathTokenizer;
    let mut report = TestReport::new(
        "extract_segment_cases",
        "Application-layer suites for json-path (ADR 0009): the parse_path and",
    );

    for case in &test_data {
        let title = new_test_title!(case.test_title);
        report.begin_case(title.clone());
        let result = extract_segment(&tokenizer, case.source);
        if result != case.expected {
            report.fail_case(format!("expected {:?} ; got {:?}", case.expected, result,));
        }
    }

    report.finish();
}

fn yield_extract_data() -> Vec<ExtractCase> {
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
