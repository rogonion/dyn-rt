//! Data-driven suites for the metadatamodel `iter` subdomain
//! (ADR 0004/0009): src/iter/{domain,application}.rs — recursive for_each /
//! filter / map walks over the model's fields/groups (port of
//! go-metadatamodel/iter, whose tests use a UserInformation model with a
//! nested Details group).

use dyn_rt_json_manipulation::application::are_equal;
use dyn_rt_metadatamodel::core::domain::{FIELDS_KEY, props};
use dyn_rt_metadatamodel::iter::{FilterDecision, Visit, filter, for_each, map};
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestReport, new_test_title};

// ---------------------------------------------------------------------------
// sample model (port of go iter's UserInformationMetadataModel, with
// section:key props and fields-as-arrays)
// ---------------------------------------------------------------------------

fn fld(path_key: &str, name: &str, data_type: &str) -> Value {
    Value::from(vec![Value::object([
        (props::GROUP_PATH_KEY, Value::from(path_key)),
        (props::GROUP_NAME, Value::from(name)),
        (props::FIELD_DATA_TYPE, Value::from(data_type)),
    ])])
}

fn fld_primary(path_key: &str, name: &str, data_type: &str) -> Value {
    Value::from(vec![Value::object([
        (props::GROUP_PATH_KEY, Value::from(path_key)),
        (props::GROUP_NAME, Value::from(name)),
        (props::FIELD_DATA_TYPE, Value::from(data_type)),
        (props::GROUP_IS_PRIMARY_KEY, Value::from(true)),
    ])])
}

fn fld_depth(path_key: &str, name: &str, data_type: &str, depth: i64) -> Value {
    Value::from(vec![Value::object([
        (props::GROUP_PATH_KEY, Value::from(path_key)),
        (props::GROUP_NAME, Value::from(name)),
        (props::FIELD_DATA_TYPE, Value::from(data_type)),
        (props::DB_JOIN_DEPTH, Value::from(depth)),
    ])])
}

fn grp(path_key: &str, name: &str, fields: Vec<(&str, Value)>, order: &[&str]) -> Value {
    Value::from(vec![Value::object([
        (props::GROUP_PATH_KEY, Value::from(path_key)),
        (props::GROUP_NAME, Value::from(name)),
        (FIELDS_KEY, Value::from(vec![Value::object(fields)])),
        (
            props::GROUP_READ_ORDER_OF_FIELDS,
            Value::from(order.iter().map(|s| Value::from(*s)).collect::<Vec<_>>()),
        ),
    ])])
}

fn str_array(items: &[&str]) -> Value {
    Value::from(items.iter().map(|s| Value::from(*s)).collect::<Vec<_>>())
}

/// Root group "Root Group" with fields ID, Name and nested group Details
/// (City, ZipCode) — port of go iter's UserInformationMetadataModel.
fn user_information_model() -> Value {
    Value::object([
        (props::GROUP_PATH_KEY, Value::from("$")),
        (props::GROUP_NAME, Value::from("Root Group")),
        (
            FIELDS_KEY,
            Value::from(vec![Value::object(vec![
                (
                    "ID",
                    fld_primary("$['group:fields'][*].ID[*]", "Primary ID", "Text"),
                ),
                (
                    "Name",
                    fld("$['group:fields'][*].Name[*]", "User Name", "Text"),
                ),
                (
                    "Details",
                    grp(
                        "$['group:fields'][*].Details[*]",
                        "Address Details",
                        vec![
                            (
                                "City",
                                fld_depth(
                                    "$['group:fields'][*].Details[*]['group:fields'][*].City[*]",
                                    "City Name",
                                    "Text",
                                    1,
                                ),
                            ),
                            (
                                "ZipCode",
                                fld_depth(
                                    "$['group:fields'][*].Details[*]['group:fields'][*].ZipCode[*]",
                                    "Postal Code",
                                    "Number",
                                    1,
                                ),
                            ),
                        ],
                        &["City", "ZipCode"],
                    ),
                ),
            ])]),
        ),
        (
            props::GROUP_READ_ORDER_OF_FIELDS,
            str_array(&["ID", "Name", "Details"]),
        ),
    ])
}

/// The model after removing fields whose name ends with "Name" (go filter
/// test case 2): root keeps ID + Details, Details keeps ZipCode.
fn filtered_no_name_suffix() -> Value {
    Value::object([
        (props::GROUP_PATH_KEY, Value::from("$")),
        (props::GROUP_NAME, Value::from("Root Group")),
        (
            FIELDS_KEY,
            Value::from(vec![Value::object(vec![
                (
                    "ID",
                    fld_primary("$['group:fields'][*].ID[*]", "Primary ID", "Text"),
                ),
                (
                    "Details",
                    grp(
                        "$['group:fields'][*].Details[*]",
                        "Address Details",
                        vec![(
                            "ZipCode",
                            fld_depth(
                                "$['group:fields'][*].Details[*]['group:fields'][*].ZipCode[*]",
                                "Postal Code",
                                "Number",
                                1,
                            ),
                        )],
                        &["ZipCode"],
                    ),
                ),
            ])]),
        ),
        (
            props::GROUP_READ_ORDER_OF_FIELDS,
            str_array(&["ID", "Details"]),
        ),
    ])
}

/// The model after removing all groups (go filter test case 3).
fn filtered_no_groups() -> Value {
    Value::object([
        (props::GROUP_PATH_KEY, Value::from("$")),
        (props::GROUP_NAME, Value::from("Root Group")),
        (
            FIELDS_KEY,
            Value::from(vec![Value::object(vec![
                (
                    "ID",
                    fld_primary("$['group:fields'][*].ID[*]", "Primary ID", "Text"),
                ),
                (
                    "Name",
                    fld("$['group:fields'][*].Name[*]", "User Name", "Text"),
                ),
            ])]),
        ),
        (
            props::GROUP_READ_ORDER_OF_FIELDS,
            str_array(&["ID", "Name"]),
        ),
    ])
}

/// The model after appending " Found" to names ending with "Name" (go map
/// test case 2).
fn mapped_name_suffix_found() -> Value {
    Value::object([
        (props::GROUP_PATH_KEY, Value::from("$")),
        (props::GROUP_NAME, Value::from("Root Group")),
        (
            FIELDS_KEY,
            Value::from(vec![Value::object(vec![
                (
                    "ID",
                    fld_primary("$['group:fields'][*].ID[*]", "Primary ID", "Text"),
                ),
                (
                    "Name",
                    fld("$['group:fields'][*].Name[*]", "User Name Found", "Text"),
                ),
                (
                    "Details",
                    grp(
                        "$['group:fields'][*].Details[*]",
                        "Address Details",
                        vec![
                            (
                                "City",
                                fld_depth(
                                    "$['group:fields'][*].Details[*]['group:fields'][*].City[*]",
                                    "City Name Found",
                                    "Text",
                                    1,
                                ),
                            ),
                            (
                                "ZipCode",
                                fld_depth(
                                    "$['group:fields'][*].Details[*]['group:fields'][*].ZipCode[*]",
                                    "Postal Code",
                                    "Number",
                                    1,
                                ),
                            ),
                        ],
                        &["City", "ZipCode"],
                    ),
                ),
            ])]),
        ),
        (
            props::GROUP_READ_ORDER_OF_FIELDS,
            str_array(&["ID", "Name", "Details"]),
        ),
    ])
}

/// The model after appending " Found" to group names (go map test case 3).
fn mapped_group_name_found() -> Value {
    let mut model = user_information_model();
    if let Some(details) = model
        .get_mut(FIELDS_KEY)
        .and_then(Value::as_array_mut)
        .and_then(|a| a.first_mut())
        .and_then(Value::as_object_mut)
        .and_then(|obj| obj.iter_mut().find(|(k, _)| k == "Details"))
        .map(|(_, entry)| entry)
        .and_then(Value::as_array_mut)
        .and_then(|a| a.first_mut())
    {
        dyn_rt_metadatamodel::core::application::set_prop(
            details,
            props::GROUP_NAME,
            Value::from("Address Details Found"),
        );
    }
    model
}

// ---------------------------------------------------------------------------
// cases
// ---------------------------------------------------------------------------

enum IterOp {
    /// for_each: count visited fields/groups
    ForEachCount { expected: usize },
    /// for_each: terminate after n visits
    ForEachTerminateAfter { n: usize, expected: usize },
    /// for_each: skip descending into groups
    ForEachSkipGroups { expected: usize },
    /// for_each: collect visited group:names in order
    ForEachNames { expected: Vec<&'static str> },
    /// filter: keep everything
    FilterKeepAll { expected: fn() -> Value },
    /// filter: remove fields whose group:name ends with "Name"
    FilterRemoveNameSuffix { expected: fn() -> Value },
    /// filter: remove all groups
    FilterRemoveGroups { expected: fn() -> Value },
    /// map: identity
    MapIdentity { expected: fn() -> Value },
    /// map: append " Found" to names ending with "Name"
    MapNameSuffixFound { expected: fn() -> Value },
    /// map: append " Found" to group names
    MapGroupNameFound { expected: fn() -> Value },
}

struct IterCase {
    test_title: Option<String>,
    op: IterOp,
}

macro_rules! iter_case {
    ($op:expr $(, test_title = $title:expr)?) => {
        IterCase {
            test_title: None $(.or(Some($title.to_string())))?,
            op: $op,
        }
    };
}

/// The props of `props` with `group:name` appended with `suffix` — helper
/// for the map callbacks; returns the new ENTRY (array with the props).
fn renamed_entry(props: &Value, suffix: &str) -> Value {
    let mut new_props = props.clone();
    if let Some(name) = props.get(props::GROUP_NAME).and_then(Value::as_str) {
        dyn_rt_metadatamodel::core::application::set_prop(
            &mut new_props,
            props::GROUP_NAME,
            Value::from(format!("{name}{suffix}")),
        );
    }
    Value::from(vec![new_props])
}

fn name_of(props: &Value) -> Option<String> {
    props
        .get(props::GROUP_NAME)
        .and_then(Value::as_str)
        .map(|s| s.to_string())
}

#[test]
fn iter_cases() {
    let test_data: Vec<IterCase> = vec![
        iter_case!(
            IterOp::ForEachCount { expected: 5 },
            test_title = "for_each visits all 5 fields/groups incl. nested (go ForEach case)"
        ),
        iter_case!(
            IterOp::ForEachTerminateAfter { n: 2, expected: 2 },
            test_title = "for_each terminates when the callback says so"
        ),
        iter_case!(
            IterOp::ForEachSkipGroups { expected: 3 },
            test_title = "for_each with skip_fields on groups does not descend"
        ),
        iter_case!(
            IterOp::ForEachNames {
                expected: vec![
                    "Primary ID",
                    "User Name",
                    "Address Details",
                    "City Name",
                    "Postal Code"
                ]
            },
            test_title = "for_each visits in read order, depth-first"
        ),
        iter_case!(
            IterOp::FilterKeepAll {
                expected: user_information_model
            },
            test_title = "filter keep everything leaves the model unchanged (go Filter case 1)"
        ),
        iter_case!(
            IterOp::FilterRemoveNameSuffix {
                expected: filtered_no_name_suffix
            },
            test_title = "filter removes fields whose name ends with 'Name' (go Filter case 2)"
        ),
        iter_case!(
            IterOp::FilterRemoveGroups {
                expected: filtered_no_groups
            },
            test_title = "filter removes groups, keeping plain fields (go Filter case 3)"
        ),
        iter_case!(
            IterOp::MapIdentity {
                expected: user_information_model
            },
            test_title = "map identity leaves the model unchanged (go Map case 1)"
        ),
        iter_case!(
            IterOp::MapNameSuffixFound {
                expected: mapped_name_suffix_found
            },
            test_title = "map appends ' Found' to names ending with 'Name' (go Map case 2)"
        ),
        iter_case!(
            IterOp::MapGroupNameFound {
                expected: mapped_group_name_found
            },
            test_title = "map appends ' Found' to group names (go Map case 3)"
        ),
    ];

    let mut report = TestReport::new(
        "iter_cases",
        "iter for_each/filter/map walks (src/iter/application.rs)",
    );
    for case in &test_data {
        report.begin_case(new_test_title!(case.test_title));
        match &case.op {
            IterOp::ForEachCount { expected } => {
                let mut count = 0usize;
                let result = for_each(&user_information_model(), &mut |_props| {
                    count += 1;
                    Visit::CONTINUE
                });
                match result {
                    Ok(()) => {
                        if count != *expected {
                            report.fail_case(format!("count: expected {expected}, got {count}"));
                        }
                    }
                    Err(err) => report.fail_case(format!("for_each failed: {err}")),
                }
            }
            IterOp::ForEachTerminateAfter { n, expected } => {
                let mut count = 0usize;
                let n = *n;
                let result = for_each(&user_information_model(), &mut |_props| {
                    count += 1;
                    if count >= n {
                        Visit::TERMINATE
                    } else {
                        Visit::CONTINUE
                    }
                });
                match result {
                    Ok(()) => {
                        if count != *expected {
                            report.fail_case(format!("count: expected {expected}, got {count}"));
                        }
                    }
                    Err(err) => report.fail_case(format!("for_each failed: {err}")),
                }
            }
            IterOp::ForEachSkipGroups { expected } => {
                let mut count = 0usize;
                let result = for_each(&user_information_model(), &mut |props| {
                    count += 1;
                    let skip = dyn_rt_metadatamodel::core::application::is_group(props);
                    Visit {
                        terminate: false,
                        skip_fields: skip,
                    }
                });
                match result {
                    Ok(()) => {
                        if count != *expected {
                            report.fail_case(format!("count: expected {expected}, got {count}"));
                        }
                    }
                    Err(err) => report.fail_case(format!("for_each failed: {err}")),
                }
            }
            IterOp::ForEachNames { expected } => {
                let mut names: Vec<String> = Vec::new();
                let result = for_each(&user_information_model(), &mut |props| {
                    names.push(name_of(props).unwrap_or_else(|| "<missing>".to_string()));
                    Visit::CONTINUE
                });
                match result {
                    Ok(()) => {
                        let expected: Vec<String> =
                            expected.iter().map(|s| s.to_string()).collect();
                        if names != expected {
                            report
                                .fail_case(format!("names: expected {expected:?}, got {names:?}"));
                        }
                    }
                    Err(err) => report.fail_case(format!("for_each failed: {err}")),
                }
            }
            IterOp::FilterKeepAll { expected }
            | IterOp::FilterRemoveNameSuffix { expected }
            | IterOp::FilterRemoveGroups { expected } => {
                let mut model = user_information_model();
                let result = filter(&mut model, &mut |props| {
                    let retain = match case.op {
                        IterOp::FilterRemoveNameSuffix { .. } => {
                            !name_of(props).is_some_and(|name| name.ends_with("Name"))
                        }
                        IterOp::FilterRemoveGroups { .. } => {
                            !dyn_rt_metadatamodel::core::application::is_group(props)
                        }
                        _ => true,
                    };
                    FilterDecision {
                        retain,
                        skip_fields: false,
                    }
                });
                match result {
                    Ok(()) => {
                        let expected = expected();
                        if !are_equal(&model, &expected) {
                            report
                                .fail_case(format!("model: expected {expected:?}, got {model:?}"));
                        }
                    }
                    Err(err) => report.fail_case(format!("filter failed: {err}")),
                }
            }
            IterOp::MapIdentity { expected }
            | IterOp::MapNameSuffixFound { expected }
            | IterOp::MapGroupNameFound { expected } => {
                let mut model = user_information_model();
                let result = map(&mut model, &mut |props| {
                    let entry = match case.op {
                        IterOp::MapNameSuffixFound { .. } => match name_of(props) {
                            Some(name) if name.ends_with("Name") => renamed_entry(props, " Found"),
                            _ => props_entry(props),
                        },
                        IterOp::MapGroupNameFound { .. } => {
                            if dyn_rt_metadatamodel::core::application::is_group(props) {
                                renamed_entry(props, " Found")
                            } else {
                                props_entry(props)
                            }
                        }
                        _ => props_entry(props),
                    };
                    (entry, false)
                });
                match result {
                    Ok(()) => {
                        let expected = expected();
                        if !are_equal(&model, &expected) {
                            report
                                .fail_case(format!("model: expected {expected:?}, got {model:?}"));
                        }
                    }
                    Err(err) => report.fail_case(format!("map failed: {err}")),
                }
            }
        }
    }
    report.finish();
}

/// Rebuilds the entry (array with the props) unchanged — the map callback's
/// "keep as-is" return.
fn props_entry(props: &Value) -> Value {
    Value::from(vec![props.clone()])
}
