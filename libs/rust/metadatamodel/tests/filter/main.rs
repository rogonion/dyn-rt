//! Data-driven suites for the metadatamodel `filter` subdomain
//! (ADR 0004/0009): src/filter/{domain,application,conditions}.rs —
//! evaluating query conditions (logical operators + per-field conditions)
//! against a root sequence in source data, returning the excluded indexes
//! (port of go-metadatamodel/filter's filter_data_test.go).

use dyn_rt_metadatamodel::core::domain::{FIELDS_KEY, props};
use dyn_rt_metadatamodel::filter::DataFilter;
use dyn_rt_metadatamodel::filter::domain::{condition, condition_props, field_type, query};
use dyn_rt_reflect::{Reflect, ReflectStruct};
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestReport, new_test_title};

// ---------------------------------------------------------------------------
// source data shapes (go's testdata.Product / testdata.UserProfile with
// slice fields, since model cardinality defaults to array)
// ---------------------------------------------------------------------------

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct ProductRow {
    #[reflect(rename = "ID")]
    pub id: Vec<i64>,
    #[reflect(rename = "Name")]
    pub name: Vec<String>,
    #[reflect(rename = "Price")]
    pub price: Vec<f64>,
    #[reflect(rename = "Created")]
    pub created: Vec<String>,
}

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct AddressRow {
    #[reflect(rename = "Street")]
    pub street: Vec<String>,
    #[reflect(rename = "City")]
    pub city: Vec<String>,
}

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct UserProfileRow {
    #[reflect(rename = "Name")]
    pub name: Vec<String>,
    #[reflect(rename = "Age")]
    pub age: Vec<i64>,
    #[reflect(rename = "Address")]
    pub address: Vec<AddressRow>,
}

// ---------------------------------------------------------------------------
// sample models (section:key props, fields-as-arrays, [*] templates)
// ---------------------------------------------------------------------------

fn fld(path_key: &str, name: &str, data_type: &str) -> Value {
    Value::from(vec![Value::object([
        (props::GROUP_PATH_KEY, Value::from(path_key)),
        (props::GROUP_NAME, Value::from(name)),
        (props::FIELD_DATA_TYPE, Value::from(data_type)),
    ])])
}

fn grp(path_key: &str, name: &str, fields: Vec<(&str, Value)>) -> Value {
    Value::from(vec![Value::object([
        (props::GROUP_PATH_KEY, Value::from(path_key)),
        (props::GROUP_NAME, Value::from(name)),
        (FIELDS_KEY, Value::from(vec![Value::object(fields)])),
    ])])
}

/// Product model with fields ID, Name, Price, Created.
fn product_model() -> Value {
    Value::object([
        (props::GROUP_PATH_KEY, Value::from("$")),
        (props::GROUP_NAME, Value::from("Product")),
        (
            FIELDS_KEY,
            Value::from(vec![Value::object(vec![
                ("ID", fld("$['group:fields'][*].ID[*]", "ID", "Number")),
                ("Name", fld("$['group:fields'][*].Name[*]", "Name", "Text")),
                (
                    "Price",
                    fld("$['group:fields'][*].Price[*]", "Price", "Number"),
                ),
                (
                    "Created",
                    fld("$['group:fields'][*].Created[*]", "Created", "Timestamp"),
                ),
            ])]),
        ),
    ])
}

/// UserProfile model with fields Name, Age and nested group Address
/// (Street, City).
fn user_profile_model() -> Value {
    Value::object([
        (props::GROUP_PATH_KEY, Value::from("$")),
        (props::GROUP_NAME, Value::from("UserProfile")),
        (
            FIELDS_KEY,
            Value::from(vec![Value::object(vec![
                ("Name", fld("$['group:fields'][*].Name[*]", "Name", "Text")),
                ("Age", fld("$['group:fields'][*].Age[*]", "Age", "Number")),
                (
                    "Address",
                    grp(
                        "$['group:fields'][*].Address[*]",
                        "Address",
                        vec![
                            (
                                "Street",
                                fld(
                                    "$['group:fields'][*].Address[*]['group:fields'][*].Street[*]",
                                    "Street",
                                    "Text",
                                ),
                            ),
                            (
                                "City",
                                fld(
                                    "$['group:fields'][*].Address[*]['group:fields'][*].City[*]",
                                    "City",
                                    "Text",
                                ),
                            ),
                        ],
                    ),
                ),
            ])]),
        ),
    ])
}

// ---------------------------------------------------------------------------
// query condition builders
// ---------------------------------------------------------------------------

fn q_logical(operator: &str, negate: bool, conditions: Vec<Value>) -> Value {
    Value::object([
        (query::TYPE, Value::from(query::TYPE_LOGICAL_OPERATOR)),
        (query::LOGICAL_OPERATOR, Value::from(operator)),
        (query::NEGATE, Value::from(negate)),
        (query::VALUE, Value::from(conditions)),
    ])
}

fn q_field_group(negate: bool, entries: Vec<(&str, Value)>) -> Value {
    Value::object([
        (query::TYPE, Value::from(query::TYPE_FIELD_GROUP)),
        (query::NEGATE, Value::from(negate)),
        (query::VALUE, Value::object(entries)),
    ])
}

fn conditions(entries: Vec<(&str, Value)>) -> Value {
    Value::object(entries)
}

fn number_cond_value(value: i64) -> Value {
    Value::object([
        (
            condition_props::ASSUMED_FIELD_TYPE,
            Value::from(field_type::NUMBER),
        ),
        (condition_props::VALUE, Value::from(value)),
    ])
}

fn text_cond(name: &str, value: &str) -> Value {
    conditions(vec![(
        name,
        Value::object([
            (
                condition_props::ASSUMED_FIELD_TYPE,
                Value::from(field_type::TEXT),
            ),
            (condition_props::VALUE, Value::from(value)),
        ]),
    )])
}

// ---------------------------------------------------------------------------
// source data
// ---------------------------------------------------------------------------

fn products() -> Box<dyn Reflect> {
    Box::new(vec![
        ProductRow {
            id: vec![0],
            name: vec!["Product 0".to_string()],
            price: vec![11.0],
            created: vec!["2023-05-01T10:00:00Z".to_string()],
        },
        ProductRow {
            id: vec![1],
            name: vec!["Product 1".to_string()],
            created: vec!["2024-06-01T10:00:00Z".to_string()],
            ..Default::default()
        },
        ProductRow {
            id: vec![2],
            name: vec!["Product 2".to_string()],
            created: vec!["2025-01-15T08:30:00Z".to_string()],
            ..Default::default()
        },
        ProductRow {
            id: vec![3],
            name: vec!["Product 3".to_string()],
            created: vec!["2022-12-31T23:59:59Z".to_string()],
            ..Default::default()
        },
    ])
}

fn user_profiles() -> Box<dyn Reflect> {
    Box::new(vec![
        UserProfileRow {
            name: vec!["User 0".to_string()],
            age: vec![10],
            address: vec![AddressRow {
                street: vec!["Street 1".to_string()],
                city: vec!["City 1".to_string()],
            }],
        },
        UserProfileRow {
            name: vec!["User 2".to_string()],
            age: vec![20],
            address: vec![AddressRow {
                street: vec!["Street 2".to_string()],
                city: vec!["City 2".to_string()],
            }],
        },
        UserProfileRow {
            name: vec!["User 3".to_string()],
            age: vec![30],
            address: vec![
                AddressRow {
                    street: vec!["Street 3".to_string()],
                    city: vec!["City 3".to_string()],
                },
                AddressRow {
                    street: vec!["Street 4".to_string()],
                    city: vec!["City 4".to_string()],
                },
            ],
        },
    ])
}

// ---------------------------------------------------------------------------
// cases (port of go's filterDataTestData)
// ---------------------------------------------------------------------------

struct FilterCase {
    test_title: Option<String>,
    source: fn() -> Box<dyn Reflect>,
    model: fn() -> Value,
    query: Value,
    root_json_path_key: &'static str,
    root_json_path_to_value: &'static str,
    expected_exclude: Vec<usize>,
    expect_error: Option<&'static str>,
    silence_errors: bool,
}

macro_rules! filter_case {
    // NB: expect_error/silence arms must come first — `expect_error = ...`
    // parses as an assignment expression otherwise
    ($source:expr, $model:expr, $query:expr, $expected:expr $(, expect_error = $err:expr)? $(, silence = $sil:expr)? $(, root_key = $rk:expr)? $(, root_to_value = $rtv:expr)? $(, test_title = $title:expr)?) => {
        FilterCase {
            test_title: None $(.or(Some($title.to_string())))?,
            source: $source,
            model: $model,
            query: $query,
            root_json_path_key: { #[allow(unused_variables)] let v: &'static str = ""; $( let v = $rk; )? v },
            root_json_path_to_value: { #[allow(unused_variables)] let v: &'static str = ""; $( let v = $rtv; )? v },
            expected_exclude: $expected,
            expect_error: None $(.or(Some($err)))?,
            silence_errors: { #[allow(unused_variables)] let v = false; $( let v = $sil; )? v },
        }
    };
}

#[test]
fn filter_data_cases() {
    let id_key = "$['group:fields'][*].ID[*]";
    let name_key = "$['group:fields'][*].Name[*]";
    let created_key = "$['group:fields'][*].Created[*]";
    let address_key = "$['group:fields'][*].Address[*]";
    let city_key = "$['group:fields'][*].Address[*]['group:fields'][*].City[*]";

    let test_data: Vec<FilterCase> = vec![
        // go case 1: Product model, AND(ID > 0 AND ID < 3, Name EndsWith "2")
        filter_case!(
            products,
            product_model,
            q_logical(
                query::AND,
                false,
                vec![
                    q_field_group(
                        false,
                        vec![(
                            id_key,
                            conditions(vec![
                                (condition::GREATER_THAN, number_cond_value(0)),
                                (condition::LESS_THAN, number_cond_value(3)),
                            ]),
                        )],
                    ),
                    q_field_group(
                        false,
                        vec![(name_key, text_cond(condition::ENDS_WITH, "2"))],
                    ),
                ],
            ),
            vec![0, 1, 3],
            test_title =
                "Product model: AND(ID>0 AND ID<3, Name EndsWith 2) excludes [0,1,3] (go case 1)"
        ),
        // go case 2: UserProfile model, OR(Address NoOfEntriesGreaterThan 1, Address.City BeginsWith "Streete")
        filter_case!(
            user_profiles,
            user_profile_model,
            q_logical(
                query::OR,
                false,
                vec![
                    q_field_group(
                        false,
                        vec![(
                            address_key,
                            conditions(vec![(
                                condition::NO_OF_ENTRIES_GREATER_THAN,
                                Value::object([(condition_props::VALUE, Value::from(1i64))]),
                            )]),
                        )],
                    ),
                    q_field_group(
                        false,
                        vec![(city_key, text_cond(condition::BEGINS_WITH, "Streete"))],
                    ),
                ],
            ),
            vec![0, 1, 2],
            test_title = "UserProfile model: OR(Address NoOfEntriesGreaterThan 1, City BeginsWith 'Streete') excludes all (go case 2)"
        ),
        // go case 3: focus on Address of profile at index 2, negated BeginsWith "City 4"
        filter_case!(
            user_profiles,
            user_profile_model,
            q_field_group(
                true,
                vec![(city_key, text_cond(condition::BEGINS_WITH, "City 4"))],
            ),
            vec![1],
            root_key = "$['group:fields'][*].Address[*]",
            root_to_value = "$[2].Address",
            test_title = "UserProfile focus on $[2].Address, negated City BeginsWith 'City 4' excludes [1] (go case 3)"
        ),
        // timestamp condition: Created GreaterThan 2024-01-01 (yyyy-mm-dd)
        filter_case!(
            products,
            product_model,
            q_field_group(
                false,
                vec![(
                    created_key,
                    conditions(vec![(
                        condition::GREATER_THAN,
                        Value::object([
                            (
                                condition_props::ASSUMED_FIELD_TYPE,
                                Value::from(field_type::TIMESTAMP)
                            ),
                            (condition_props::DATE_TIME_FORMAT, Value::from("yyyy-mm-dd")),
                            (condition_props::VALUE, Value::from("2024-01-01T00:00:00Z")),
                        ]),
                    )]),
                )],
            ),
            vec![0, 3],
            test_title = "timestamp condition: Created > 2024-01-01 (yyyy-mm-dd) excludes [0,3]"
        ),
        // text condition with Values array + CaseInsensitive, negated at field level
        filter_case!(
            products,
            product_model,
            q_field_group(
                false,
                vec![(
                    name_key,
                    conditions(vec![(
                        condition::EQUAL_TO,
                        Value::object([
                            (
                                condition_props::ASSUMED_FIELD_TYPE,
                                Value::from(field_type::TEXT)
                            ),
                            (condition_props::CASE_INSENSITIVE, Value::from(true)),
                            (
                                condition_props::VALUES,
                                Value::from(vec![
                                    Value::from("product 1"),
                                    Value::from("PRODUCT 3"),
                                ]),
                            ),
                        ]),
                    )]),
                )],
            ),
            vec![0, 2],
            test_title = "text EqualTo with Values array, case-insensitive, excludes [0,2]"
        ),
        // unknown query condition type is an error (or false when silenced)
        filter_case!(
            products,
            product_model,
            Value::object([(query::TYPE, Value::from("Nope"))]),
            vec![],
            expect_error = "unknown query condition type",
            test_title = "unknown query condition type errors"
        ),
        filter_case!(
            products,
            product_model,
            Value::object([(query::TYPE, Value::from("Nope"))]),
            vec![0, 1, 2, 3],
            silence = true,
            test_title = "silenced errors make the condition false, excluding all"
        ),
    ];

    let mut report = TestReport::new(
        "filter_data_cases",
        "filter DataFilter query-condition evaluation (src/filter)",
    );
    for case in &test_data {
        report.begin_case(new_test_title!(case.test_title));
        let source = (case.source)();
        let model = (case.model)();
        let mut data_filter = DataFilter::new(source.as_ref(), &model);
        data_filter.silence_errors = case.silence_errors;
        match data_filter.filter(
            &case.query,
            case.root_json_path_key,
            case.root_json_path_to_value,
        ) {
            Ok(exclude) => match &case.expect_error {
                Some(needle) => {
                    report.fail_case(format!(
                        "expected error containing {needle:?}, got Ok({exclude:?})"
                    ));
                }
                None => {
                    if exclude != case.expected_exclude {
                        report.fail_case(format!(
                            "exclude indexes: expected {:?}, got {:?}",
                            case.expected_exclude, exclude
                        ));
                    }
                }
            },
            Err(err) => match &case.expect_error {
                Some(needle) => {
                    if !err.to_string().contains(needle) {
                        report
                            .fail_case(format!("error expected to contain {needle:?}, got {err}"));
                    }
                }
                None => report.fail_case(format!("unexpected error: {err}")),
            },
        }
    }
    report.finish();
}
