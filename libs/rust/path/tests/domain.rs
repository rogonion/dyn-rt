//! Domain-layer suite for path (ADR 0009): Display rendering of the AST,
//! built with the `json_path_*_segment!` / `new_recursive_descent_segment(s)!`
//! constructor macros so the table reads like the paths themselves.

mod common;

use common::{RenderCase, render_case};
use dyn_rt_path::domain::AccessStrategy;
use dyn_rt_path::{
    json_path_index_selector_segment, json_path_key_selector_segment, json_path_root_segment,
    json_path_slice_selector_segment, json_path_union_selector_segment, json_path_wildcard_segment,
    new_recursive_descent_segment, new_recursive_descent_segments,
};
use dyn_rt_test_support::{TestReport, new_test_title};

#[test]
fn ast_render_roundtrip() {
    let test_data = yield_render_data();
    let mut report = TestReport::new(
        "ast_render_roundtrip",
        "Domain-layer suite for path (ADR 0009): Display rendering of the AST,",
    );

    for case in &test_data {
        let title = new_test_title!(case.test_title);
        report.begin_case(title.clone());
        let rendered = case.segments.to_string();
        if rendered != case.roundtrip {
            report.fail_case(format!("expected {:?}, got {:?}", case.roundtrip, rendered,));
        }
    }

    report.finish();
}

fn yield_render_data() -> Vec<RenderCase> {
    vec![
        render_case!(
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
        render_case!(
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
        render_case!(
            new_recursive_descent_segments!(
                new_recursive_descent_segment!(json_path_root_segment!()),
                new_recursive_descent_segment!(json_path_key_selector_segment!("store")),
                new_recursive_descent_segment!(json_path_key_selector_segment!("book")),
                new_recursive_descent_segment!(json_path_wildcard_segment!())
            ),
            roundtrip = "$..store..book..*",
            test_title = "multiple recursive descents ending in wildcard"
        ),
        render_case!(
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
        render_case!(
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
        render_case!(
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
        render_case!(
            new_recursive_descent_segments!(new_recursive_descent_segment!(
                json_path_root_segment!(),
                json_path_key_selector_segment!("transactions"),
                json_path_slice_selector_segment!(1:5:2),
                json_path_key_selector_segment!("amount")
            )),
            roundtrip = "$.transactions[1:5:2].amount",
            test_title = "slice selector"
        ),
        render_case!(
            new_recursive_descent_segments!(new_recursive_descent_segment!(
                json_path_root_segment!(),
                json_path_key_selector_segment!("transactions"),
                json_path_slice_selector_segment!(1:-5),
                json_path_key_selector_segment!("amount")
            )),
            roundtrip = "$.transactions[1:-5].amount",
            test_title = "slice selector with negative end"
        ),
        render_case!(
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
        render_case!(
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
        render_case!(
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
        render_case!(
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
        render_case!(
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
        render_case!(
            new_recursive_descent_segments!(new_recursive_descent_segment!(
                json_path_root_segment!(),
                json_path_key_selector_segment!("data"),
                json_path_wildcard_segment!(),
                json_path_key_selector_segment!("price")
            )),
            roundtrip = "$.data.*.price",
            test_title = "wildcard member"
        ),
        render_case!(
            new_recursive_descent_segments!(new_recursive_descent_segment!(
                json_path_root_segment!(),
                json_path_key_selector_segment!("store"),
                json_path_key_selector_segment!("bicycle"),
                json_path_key_selector_segment!("item-code")
            )),
            roundtrip = "$.store.bicycle['item-code']",
            test_title = "bracketed property key"
        ),
        render_case!(
            new_recursive_descent_segments!(
                new_recursive_descent_segment!(json_path_root_segment!()),
                new_recursive_descent_segment!(json_path_key_selector_segment!("author"))
            ),
            roundtrip = "$..author",
            test_title = "single recursive descent"
        ),
        render_case!(
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
        render_case!(
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
        render_case!(
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
        render_case!(
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
        render_case!(
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
        render_case!(
            new_recursive_descent_segments!(new_recursive_descent_segment!(
                json_path_root_segment!(),
                json_path_key_selector_segment!("transactions"),
                json_path_slice_selector_segment!(1:5:2),
                json_path_wildcard_segment!()
            )),
            roundtrip = "$.transactions[1:5:2].*",
            test_title = "slice followed by wildcard (js variant)"
        ),
        render_case!(
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
        render_case!(
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
