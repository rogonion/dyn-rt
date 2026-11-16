//! Adapter suite for `src/adapters.rs`'s RegexPathTokenizer: the three split
//! steps of the tokenizer port.

use crate::common::{SplitCase, split_case};
use dyn_rt_path::RegexPathTokenizer;
use dyn_rt_path::domain::PathTokenizer;
use dyn_rt_test_support::{TestReport, new_test_title};

#[test]
fn split_recursive_descent() {
    let test_data = yield_split_recursive_descent();
    let tokenizer = RegexPathTokenizer;
    let mut report = TestReport::new(
        "split_recursive_descent",
        "Adapter suite for `src/adapters.rs`'s RegexPathTokenizer: the three split",
    );

    for case in &test_data {
        let title = new_test_title!(case.test_title);
        report.begin_case(title.clone());
        let result: Vec<&str> = tokenizer.split_recursive_descent(case.source);
        if result != case.expected {
            report.fail_case(format!("expected {:?} ; got {:?}", case.expected, result,));
        }
    }

    report.finish();
}

fn yield_split_recursive_descent() -> Vec<SplitCase> {
    vec![
        split_case!(
            "$.store..book[0].title",
            "$.store",
            "book[0].title",
            test_title = "basic path with some recursive descent"
        ),
        split_case!(
            "$..store..book..*",
            "$",
            "store",
            "book",
            "*",
            test_title = "multiple recursive descents starting with $.."
        ),
        split_case!(
            "$['a..b']..['c..d']",
            "$['a..b']",
            "['c..d']",
            test_title = "string literals containing .. inside brackets"
        ),
        split_case!(
            "$['report-data']..['total.sum..()&^']..store.bicycle['item-code']",
            "$['report-data']",
            "['total.sum..()&^']",
            "store.bicycle['item-code']",
            test_title = "complex symbols with .. inside bracket quotes"
        ),
        split_case!(
            "$['report-data']..['total.sum']..store.bicycle['item-code']",
            "$['report-data']",
            "['total.sum']",
            "store.bicycle['item-code']",
            test_title = "multiple recursive descents with hyphenated keys"
        ),
        split_case!(
            "$.transactions[1:5:2].amount",
            "$.transactions[1:5:2].amount",
            test_title = "slice selector"
        ),
        split_case!(
            "$.data.user.preferences['theme-settings',\"font-size\",3]",
            "$.data.user.preferences['theme-settings',\"font-size\",3]",
            test_title = "union selector with mixed types"
        ),
        split_case!(
            "$..['1st_category'].name",
            "$",
            "['1st_category'].name",
            test_title = "recursive descent into bracketed numeric key"
        ),
        split_case!(
            "$.products['item-details..'].dimensions[0].width.dimensions[2][3].width",
            "$.products['item-details..'].dimensions[0].width.dimensions[2][3].width",
            test_title = "dots inside property key + chained array indices"
        ),
        split_case!(
            "$['user info']['address.wind'][1].street",
            "$['user info']['address.wind'][1].street",
            test_title = "spaces and dots in quoted key"
        ),
        split_case!(
            "$.data[*].price",
            "$.data[*].price",
            test_title = "wildcard array match"
        ),
        split_case!(
            "$.store.bicycle['item-code']",
            "$.store.bicycle['item-code']",
            test_title = "bracketed property key"
        ),
        split_case!(
            "$..author",
            "$",
            "author",
            test_title = "single recursive descent"
        ),
        split_case!(
            "$.store.book[0].title",
            "$.store.book[0].title",
            test_title = "standard dot and index path"
        ),
        split_case!(
            "$['store\\'s..item']..id",
            "$['store\\'s..item']",
            "id",
            test_title = "escaped single quote inside key containing .."
        ),
        split_case!(
            "$[\"user\\\"name..data\"]..title",
            "$[\"user\\\"name..data\"]",
            "title",
            test_title = "escaped double quote inside key containing .."
        ),
        split_case!(
            "$['']..author",
            "$['']",
            "author",
            test_title = "empty bracketed string key"
        ),
        split_case!(
            "$['title_\"main\"..part']..id",
            "$['title_\"main\"..part']",
            "id",
            test_title = "mixed quotes inside single-quoted key with .."
        ),
    ]
}

#[test]
fn split_dot_notation() {
    let test_data = yield_split_dot_notation();
    let tokenizer = RegexPathTokenizer;
    let mut report = TestReport::new(
        "split_dot_notation",
        "Adapter suite for `src/adapters.rs`'s RegexPathTokenizer: the three split",
    );

    for case in &test_data {
        let title = new_test_title!(case.test_title);
        report.begin_case(title.clone());
        let result: Vec<&str> = tokenizer.split_dot_notation(case.source);
        if result != case.expected {
            report.fail_case(format!("expected {:?} ; got {:?}", case.expected, result,));
        }
    }

    report.finish();
}

fn yield_split_dot_notation() -> Vec<SplitCase> {
    vec![
        split_case!("$.store", "$", "store", test_title = "root and simple key"),
        split_case!(
            "book[0].title",
            "book[0]",
            "title",
            test_title = "bracket index attached to key + property title"
        ),
        split_case!("$", "$", test_title = "standalone root selector"),
        split_case!("store", "store", test_title = "standalone property name"),
        split_case!("*", "*", test_title = "standalone wildcard selector"),
        split_case!(
            "$['a..b']",
            "$['a..b']",
            test_title = "root with quoted bracket key containing inner dots"
        ),
        split_case!(
            "['c..d']",
            "['c..d']",
            test_title = "standalone quoted bracket key with inner dots"
        ),
        split_case!(
            "$['report-data']",
            "$['report-data']",
            test_title = "root with hyphenated quoted key"
        ),
        split_case!(
            "['total.sum..()&^']",
            "['total.sum..()&^']",
            test_title = "complex symbols and dots inside single quotes"
        ),
        split_case!(
            "store.bicycle['item-code']",
            "store",
            "bicycle['item-code']",
            test_title = "chained dot property with bracketed property key"
        ),
        split_case!(
            "$.transactions[1:5:2].amount",
            "$",
            "transactions[1:5:2]",
            "amount",
            test_title = "slice selector attached to property name"
        ),
        split_case!(
            "$.data.user.preferences['theme-settings',\"font-size\",3]",
            "$",
            "data",
            "user",
            "preferences['theme-settings',\"font-size\",3]",
            test_title = "deeply nested property path with mixed union selector"
        ),
        split_case!(
            "['1st_category'].name",
            "['1st_category']",
            "name",
            test_title = "bracketed category identifier followed by dot property"
        ),
        split_case!(
            "$.products['item-details..'].dimensions[0].width.dimensions[2][3].width",
            "$",
            "products['item-details..']",
            "dimensions[0]",
            "width",
            "dimensions[2][3]",
            "width",
            test_title = "complex path with dots in quotes, array indices, chained brackets"
        ),
        split_case!(
            "$['user info']['address.wind'][1].street",
            "$['user info']['address.wind'][1]",
            "street",
            test_title = "spaces and dots inside quoted brackets with array indexing"
        ),
        split_case!(
            "$.data[*].price",
            "$",
            "data[*]",
            "price",
            test_title = "wildcard array index attached to property"
        ),
        split_case!("author", "author", test_title = "single simple identifier"),
        split_case!(
            "$['store\\'s..item']",
            "$['store\\'s..item']",
            test_title = "escaped single quote and dots inside bracket key"
        ),
        split_case!(
            "$[\"user\\\"name..data\"]",
            "$[\"user\\\"name..data\"]",
            test_title = "escaped double quote and dots inside bracket key"
        ),
        split_case!(
            "$['']",
            "$['']",
            test_title = "empty single-quoted string bracket key"
        ),
        split_case!(
            "$['title_\"main\"..part']",
            "$['title_\"main\"..part']",
            test_title = "mixed double quotes and dots inside single-quoted bracket key"
        ),
    ]
}

#[test]
fn split_brackets() {
    let test_data = yield_split_brackets();
    let tokenizer = RegexPathTokenizer;
    let mut report = TestReport::new(
        "split_brackets",
        "Adapter suite for `src/adapters.rs`'s RegexPathTokenizer: the three split",
    );

    for case in &test_data {
        let title = new_test_title!(case.test_title);
        report.begin_case(title.clone());
        let result: Vec<&str> = tokenizer.split_brackets(case.source);
        if result != case.expected {
            report.fail_case(format!("expected {:?} ; got {:?}", case.expected, result,));
        }
    }

    report.finish();
}

fn yield_split_brackets() -> Vec<SplitCase> {
    vec![
        split_case!("$", "$", test_title = "standalone root selector"),
        split_case!("store", "store", test_title = "standalone property name"),
        split_case!("*", "*", test_title = "standalone wildcard selector"),
        split_case!(
            "$['a..b']",
            "$",
            "'a..b'",
            test_title = "root split from quoted bracket key"
        ),
        split_case!(
            "['c..d']",
            "'c..d'",
            test_title = "standalone quoted bracket key with inner dots"
        ),
        split_case!(
            "$['report-data']",
            "$",
            "'report-data'",
            test_title = "root split from hyphenated quoted key"
        ),
        split_case!(
            "['total.sum..()&^']",
            "'total.sum..()&^'",
            test_title = "complex symbols and dots inside single quotes"
        ),
        split_case!(
            "bicycle['item-code']",
            "bicycle",
            "'item-code'",
            test_title = "chained dot property with bracketed property key"
        ),
        split_case!(
            "transactions[1:5:2]",
            "transactions",
            "1:5:2",
            test_title = "slice selector split from property name"
        ),
        split_case!(
            "preferences['theme-settings',\"font-size\",3]",
            "preferences",
            "'theme-settings',\"font-size\",3",
            test_title = "mixed union selector split from property name"
        ),
        split_case!(
            "['1st_category']",
            "'1st_category'",
            test_title = "bracketed category identifier"
        ),
        split_case!(
            "products['item-details..']",
            "products",
            "'item-details..'",
            test_title = "property key containing dots"
        ),
        split_case!(
            "dimensions[0]",
            "dimensions",
            "0",
            test_title = "single array index"
        ),
        split_case!(
            "dimensions[2][3]",
            "dimensions",
            "2",
            "3",
            test_title = "chained array indices"
        ),
        split_case!(
            "$['user info']['address.wind'][1]",
            "$",
            "'user info'",
            "'address.wind'",
            "1",
            test_title = "chained bracket keys into individual atomic brackets"
        ),
        split_case!(
            "data[*]",
            "data",
            "*",
            test_title = "wildcard array index split from property"
        ),
        split_case!("author", "author", test_title = "single simple identifier"),
        split_case!(
            "$['store\\'s..item']",
            "$",
            "'store\\'s..item'",
            test_title = "root split from escaped quote bracket key"
        ),
        split_case!(
            "$[\"user\\\"name..data\"]",
            "$",
            "\"user\\\"name..data\"",
            test_title = "root split from escaped double quote bracket key"
        ),
        split_case!(
            "$['']",
            "$",
            "''",
            test_title = "root split from empty string bracket key"
        ),
        split_case!(
            "$['title_\"main\"..part']",
            "$",
            "'title_\"main\"..part'",
            test_title = "root split from mixed quote bracket key"
        ),
    ]
}
