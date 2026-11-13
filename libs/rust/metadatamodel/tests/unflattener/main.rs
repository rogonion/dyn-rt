//! Data-driven suites for the metadatamodel `unflattener` subdomain
//! (ADR 0004/0009): src/unflattener/{domain,application}.rs — converting a
//! 2D table back into a nested collection (port of
//! go-metadatamodel/unflattener's unflatten_test.go), incl.
//! cardinality-aware writes (single-cardinality unwrap, multi-value error /
//! take-first) and a flatten->unflatten round-trip.

use dyn_rt_json_manipulation::application::are_equal;
use dyn_rt_json_schema::{Schema, SchemaNode};
use dyn_rt_metadatamodel::core::domain::{FIELDS_KEY, props};
use dyn_rt_metadatamodel::flattener::Flattener;
use dyn_rt_metadatamodel::unflattener::{SingleCellMode, Unflattener};
use dyn_rt_reflect::ReflectStruct;
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestReport, new_test_title};

// ---------------------------------------------------------------------------
// destination shapes (same as the flattener suite's source shapes)
// ---------------------------------------------------------------------------

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct UserRow {
    #[reflect(rename = "ID")]
    pub id: Vec<i64>,
    #[reflect(rename = "Name")]
    pub name: Vec<String>,
    #[reflect(rename = "Email")]
    pub email: Vec<String>,
}

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct AddressRow {
    #[reflect(rename = "Street")]
    pub street: Vec<String>,
    #[reflect(rename = "City")]
    pub city: Vec<String>,
    #[reflect(rename = "ZipCode")]
    pub zip: Vec<String>,
}

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct ProfileRow {
    #[reflect(rename = "Name")]
    pub name: Vec<String>,
    #[reflect(rename = "Age")]
    pub age: Vec<i64>,
    #[reflect(rename = "Address")]
    pub address: Vec<AddressRow>,
}

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct EmployeeRow {
    #[reflect(rename = "ID")]
    pub id: Vec<i64>,
    #[reflect(rename = "Skills")]
    pub skills: Vec<String>,
    #[reflect(rename = "Profile")]
    pub profile: Vec<ProfileRow>,
}

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct UserWithBio {
    #[reflect(rename = "ID")]
    pub id: Vec<i64>,
    #[reflect(rename = "Bio")]
    pub bio: bool,
}

// ---------------------------------------------------------------------------
// schemas for the destinations
// ---------------------------------------------------------------------------

fn int_seq() -> SchemaNode {
    SchemaNode::sequence_node::<Vec<i64>>(Schema::Node(SchemaNode::scalar::<i64>()))
}

fn str_seq() -> SchemaNode {
    SchemaNode::sequence_node::<Vec<String>>(Schema::Node(SchemaNode::scalar::<String>()))
}

fn user_schema() -> SchemaNode {
    SchemaNode::sequence_node::<Vec<UserRow>>(Schema::Node(SchemaNode::struct_node::<UserRow>(
        vec![
            ("ID", Schema::Node(int_seq())),
            ("Name", Schema::Node(str_seq())),
            ("Email", Schema::Node(str_seq())),
        ],
    )))
}

fn address_schema() -> SchemaNode {
    SchemaNode::struct_node::<AddressRow>(vec![
        ("Street", Schema::Node(str_seq())),
        ("City", Schema::Node(str_seq())),
        ("ZipCode", Schema::Node(str_seq())),
    ])
}

fn profile_schema() -> SchemaNode {
    SchemaNode::struct_node::<ProfileRow>(vec![
        ("Name", Schema::Node(str_seq())),
        ("Age", Schema::Node(int_seq())),
        (
            "Address",
            Schema::Node(SchemaNode::sequence_node::<Vec<AddressRow>>(Schema::Node(
                address_schema(),
            ))),
        ),
    ])
}

fn employee_schema() -> SchemaNode {
    SchemaNode::sequence_node::<Vec<EmployeeRow>>(Schema::Node(SchemaNode::struct_node::<
        EmployeeRow,
    >(vec![
        ("ID", Schema::Node(int_seq())),
        ("Skills", Schema::Node(str_seq())),
        (
            "Profile",
            Schema::Node(SchemaNode::sequence_node::<Vec<ProfileRow>>(Schema::Node(
                profile_schema(),
            ))),
        ),
    ])))
}

fn user_bio_schema() -> SchemaNode {
    SchemaNode::sequence_node::<Vec<UserWithBio>>(Schema::Node(SchemaNode::struct_node::<
        UserWithBio,
    >(vec![
        ("ID", Schema::Node(int_seq())),
        ("Bio", Schema::Node(SchemaNode::scalar::<bool>())),
    ])))
}

// ---------------------------------------------------------------------------
// sample models (same shapes as the flattener suite)
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

fn fld_single(path_key: &str, name: &str, data_type: &str) -> Value {
    Value::from(vec![Value::object([
        (props::GROUP_PATH_KEY, Value::from(path_key)),
        (props::GROUP_NAME, Value::from(name)),
        (props::FIELD_DATA_TYPE, Value::from(data_type)),
        (props::FIELD_CARDINALITY, Value::from("single")),
    ])])
}

fn grp(path_key: &str, name: &str, fields: Vec<(&str, Value)>) -> Value {
    Value::from(vec![Value::object([
        (props::GROUP_PATH_KEY, Value::from(path_key)),
        (props::GROUP_NAME, Value::from(name)),
        (FIELDS_KEY, Value::from(vec![Value::object(fields)])),
    ])])
}

fn user_model() -> Value {
    Value::object([
        (props::GROUP_PATH_KEY, Value::from("$")),
        (props::GROUP_NAME, Value::from("User")),
        (
            FIELDS_KEY,
            Value::from(vec![Value::object(vec![
                (
                    "ID",
                    fld_primary("$['group:fields'][*].ID[*]", "ID", "Number"),
                ),
                ("Name", fld("$['group:fields'][*].Name[*]", "Name", "Text")),
                (
                    "Email",
                    fld("$['group:fields'][*].Email[*]", "Email", "Text"),
                ),
            ])]),
        ),
    ])
}

fn employee_model() -> Value {
    Value::object([
        (props::GROUP_PATH_KEY, Value::from("$")),
        (props::GROUP_NAME, Value::from("Employee")),
        (
            FIELDS_KEY,
            Value::from(vec![Value::object(vec![
                (
                    "ID",
                    fld_primary("$['group:fields'][*].ID[*]", "ID", "Number"),
                ),
                (
                    "Profile",
                    grp(
                        "$['group:fields'][*].Profile[*]",
                        "Profile",
                        vec![
                            (
                                "Name",
                                fld(
                                    "$['group:fields'][*].Profile[*]['group:fields'][*].Name[*]",
                                    "Name",
                                    "Text",
                                ),
                            ),
                            (
                                "Age",
                                fld(
                                    "$['group:fields'][*].Profile[*]['group:fields'][*].Age[*]",
                                    "Age",
                                    "Number",
                                ),
                            ),
                            (
                                "Address",
                                grp(
                                    "$['group:fields'][*].Profile[*]['group:fields'][*].Address[*]",
                                    "Address",
                                    vec![
                                        (
                                            "Street",
                                            fld(
                                                "$['group:fields'][*].Profile[*]['group:fields'][*].Address[*]['group:fields'][*].Street[*]",
                                                "Street",
                                                "Text",
                                            ),
                                        ),
                                        (
                                            "City",
                                            fld(
                                                "$['group:fields'][*].Profile[*]['group:fields'][*].Address[*]['group:fields'][*].City[*]",
                                                "City",
                                                "Text",
                                            ),
                                        ),
                                        (
                                            "ZipCode",
                                            fld(
                                                "$['group:fields'][*].Profile[*]['group:fields'][*].Address[*]['group:fields'][*].ZipCode[*]",
                                                "ZipCode",
                                                "Text",
                                            ),
                                        ),
                                    ],
                                ),
                            ),
                        ],
                    ),
                ),
                (
                    "Skills",
                    fld("$['group:fields'][*].Skills[*]", "Skills", "Text"),
                ),
            ])]),
        ),
    ])
}

fn user_bio_model() -> Value {
    Value::object([
        (props::GROUP_PATH_KEY, Value::from("$")),
        (props::GROUP_NAME, Value::from("User")),
        (
            FIELDS_KEY,
            Value::from(vec![Value::object(vec![
                (
                    "ID",
                    fld_primary("$['group:fields'][*].ID[*]", "ID", "Number"),
                ),
                (
                    "Bio",
                    fld_single("$['group:fields'][*].Bio[*]", "Bio", "Boolean"),
                ),
            ])]),
        ),
    ])
}

// ---------------------------------------------------------------------------
// table helpers (cells are arrays — the flattener's output formality)
// ---------------------------------------------------------------------------

fn cells_i(values: Vec<i64>) -> Value {
    Value::from(values.into_iter().map(Value::from).collect::<Vec<_>>())
}

fn cells_s(values: Vec<&str>) -> Value {
    Value::from(values.into_iter().map(Value::from).collect::<Vec<_>>())
}

fn row(cells: Vec<Value>) -> Vec<Value> {
    cells
}

// ---------------------------------------------------------------------------
// cases
// ---------------------------------------------------------------------------

enum UnflattenOp {
    /// unflatten into Vec<UserRow>
    Users {
        table: Vec<Vec<Value>>,
        expected: Vec<UserRow>,
    },
    /// unflatten into Vec<EmployeeRow>
    Employees {
        table: Vec<Vec<Value>>,
        expected: Vec<EmployeeRow>,
    },
    /// unflatten into Vec<UserWithBio> (single-cardinality field)
    Bio {
        table: Vec<Vec<Value>>,
        expected: Option<Vec<UserWithBio>>,
        expect_error: Option<&'static str>,
        mode: SingleCellMode,
    },
}

struct UnflattenCase {
    test_title: Option<String>,
    model: fn() -> Value,
    op: UnflattenOp,
}

macro_rules! unflatten_case {
    ($model:expr, $op:expr $(, test_title = $title:expr)?) => {
        UnflattenCase {
            test_title: None $(.or(Some($title.to_string())))?,
            model: $model,
            op: $op,
        }
    };
}

#[test]
fn unflatten_cases() {
    let test_data: Vec<UnflattenCase> = vec![
        // go case 1: simple user
        unflatten_case!(
            user_model,
            UnflattenOp::Users {
                table: vec![row(vec![
                    cells_i(vec![101]),
                    cells_s(vec!["Alice"]),
                    cells_s(vec!["alice@example.com"]),
                ])],
                expected: vec![UserRow {
                    id: vec![101],
                    name: vec!["Alice".to_string()],
                    email: vec!["alice@example.com".to_string()],
                }],
            },
            test_title = "simple user table unflattens to a one-element collection (go case 1)"
        ),
        // go case 2-like: nested employee, one row
        unflatten_case!(
            employee_model,
            UnflattenOp::Employees {
                table: vec![row(vec![
                    cells_i(vec![500]),
                    cells_s(vec!["Bob"]),
                    cells_i(vec![30]),
                    cells_s(vec!["123 Tech Ln"]),
                    cells_s(vec!["Silicon Valley"]),
                    cells_s(vec!["94000"]),
                    cells_s(vec!["Go", "Rust"]),
                ])],
                expected: vec![EmployeeRow {
                    id: vec![500],
                    skills: vec!["Go".to_string(), "Rust".to_string()],
                    profile: vec![ProfileRow {
                        name: vec!["Bob".to_string()],
                        age: vec![30],
                        address: vec![AddressRow {
                            street: vec!["123 Tech Ln".to_string()],
                            city: vec!["Silicon Valley".to_string()],
                            zip: vec!["94000".to_string()],
                        }],
                    }],
                }],
            },
            test_title = "nested employee row unflattens into nested groups (go case 2)"
        ),
        // cartesian table: rows merge into instances by signature
        unflatten_case!(
            employee_model,
            UnflattenOp::Employees {
                table: vec![
                    row(vec![
                        cells_i(vec![100]),
                        cells_s(vec!["Dev"]),
                        cells_i(vec![30]),
                        cells_s(vec!["Home St"]),
                        cells_s(vec!["Nairobi"]),
                        cells_s(vec!["00100"]),
                        cells_s(vec!["Go"]),
                    ]),
                    row(vec![
                        cells_i(vec![100]),
                        cells_s(vec!["Dev"]),
                        cells_i(vec![30]),
                        cells_s(vec!["Work Ave"]),
                        cells_s(vec!["Westlands"]),
                        cells_s(vec!["00200"]),
                        cells_s(vec!["Go"]),
                    ]),
                    row(vec![
                        cells_i(vec![200]),
                        cells_s(vec!["Admin"]),
                        cells_i(vec![40]),
                        cells_s(vec!["Office Rd"]),
                        cells_s(vec!["Mombasa"]),
                        cells_s(vec!["80100"]),
                        cells_s(vec!["Management"]),
                    ]),
                    row(vec![
                        cells_i(vec![200]),
                        cells_s(vec!["Consultant"]),
                        cells_i(vec![35]),
                        cells_s(vec!["Remote"]),
                        cells_s(vec!["Eldoret"]),
                        cells_s(vec!["30100"]),
                        cells_s(vec!["Management"]),
                    ]),
                ],
                expected: vec![
                    EmployeeRow {
                        id: vec![100],
                        skills: vec!["Go".to_string()],
                        profile: vec![ProfileRow {
                            name: vec!["Dev".to_string()],
                            age: vec![30],
                            address: vec![
                                AddressRow {
                                    street: vec!["Home St".to_string()],
                                    city: vec!["Nairobi".to_string()],
                                    zip: vec!["00100".to_string()],
                                },
                                AddressRow {
                                    street: vec!["Work Ave".to_string()],
                                    city: vec!["Westlands".to_string()],
                                    zip: vec!["00200".to_string()],
                                },
                            ],
                        }],
                    },
                    EmployeeRow {
                        id: vec![200],
                        skills: vec!["Management".to_string()],
                        profile: vec![
                            ProfileRow {
                                name: vec!["Admin".to_string()],
                                age: vec![40],
                                address: vec![AddressRow {
                                    street: vec!["Office Rd".to_string()],
                                    city: vec!["Mombasa".to_string()],
                                    zip: vec!["80100".to_string()],
                                }],
                            },
                            ProfileRow {
                                name: vec!["Consultant".to_string()],
                                age: vec![35],
                                address: vec![AddressRow {
                                    street: vec!["Remote".to_string()],
                                    city: vec!["Eldoret".to_string()],
                                    zip: vec!["30100".to_string()],
                                }],
                            },
                        ],
                    },
                ],
            },
            test_title =
                "cartesian table merges rows into instances by signature (go case 5 inverse)"
        ),
        // single-cardinality field: cell array unwrapped to the scalar
        unflatten_case!(
            user_bio_model,
            UnflattenOp::Bio {
                table: vec![row(vec![
                    cells_i(vec![7]),
                    Value::from(vec![Value::from(true)])
                ])],
                expected: Some(vec![UserWithBio {
                    id: vec![7],
                    bio: true,
                }]),
                expect_error: None,
                mode: SingleCellMode::Error,
            },
            test_title = "single-cardinality field: cell array unwrapped to the scalar"
        ),
        // single-cardinality field with >1 values: error in strict mode
        unflatten_case!(
            user_bio_model,
            UnflattenOp::Bio {
                table: vec![row(vec![
                    cells_i(vec![7]),
                    Value::from(vec![Value::from(true), Value::from(false)])
                ])],
                expected: None,
                expect_error: Some("single-cardinality"),
                mode: SingleCellMode::Error,
            },
            test_title = "single-cardinality field with 2 values errors in strict mode"
        ),
        // same but TakeFirst
        unflatten_case!(
            user_bio_model,
            UnflattenOp::Bio {
                table: vec![row(vec![
                    cells_i(vec![7]),
                    Value::from(vec![Value::from(true), Value::from(false)])
                ])],
                expected: Some(vec![UserWithBio {
                    id: vec![7],
                    bio: true,
                }]),
                expect_error: None,
                mode: SingleCellMode::TakeFirst,
            },
            test_title = "single-cardinality field with 2 values takes the first in TakeFirst mode"
        ),
    ];

    let mut report = TestReport::new(
        "unflatten_cases",
        "unflattener: 2D table to nested collection (src/unflattener)",
    );
    for case in &test_data {
        report.begin_case(new_test_title!(case.test_title));
        let model = (case.model)();
        match &case.op {
            UnflattenOp::Users { table, expected } => {
                let schema = Schema::Node(user_schema());
                let mut dest: Vec<UserRow> = Vec::new();
                let mut unflattener = match Unflattener::new(&model, &mut dest, Some(&schema)) {
                    Ok(u) => u,
                    Err(err) => {
                        report.fail_case(format!("new failed: {err}"));
                        continue;
                    }
                };
                match unflattener.unflatten(table) {
                    Ok(()) => {
                        if !are_equal(&dest, expected) {
                            report.fail_case(format!("dest: expected {expected:?}, got {dest:?}"));
                        }
                    }
                    Err(err) => report.fail_case(format!("unflatten failed: {err}")),
                }
            }
            UnflattenOp::Employees { table, expected } => {
                let schema = Schema::Node(employee_schema());
                let mut dest: Vec<EmployeeRow> = Vec::new();
                let mut unflattener = match Unflattener::new(&model, &mut dest, Some(&schema)) {
                    Ok(u) => u,
                    Err(err) => {
                        report.fail_case(format!("new failed: {err}"));
                        continue;
                    }
                };
                match unflattener.unflatten(table) {
                    Ok(()) => {
                        if !are_equal(&dest, expected) {
                            report.fail_case(format!("dest: expected {expected:?}, got {dest:?}"));
                        }
                    }
                    Err(err) => report.fail_case(format!("unflatten failed: {err}")),
                }
            }
            UnflattenOp::Bio {
                table,
                expected,
                expect_error,
                mode,
            } => {
                let schema = Schema::Node(user_bio_schema());
                let mut dest: Vec<UserWithBio> = Vec::new();
                let mut unflattener = match Unflattener::new(&model, &mut dest, Some(&schema)) {
                    Ok(u) => u,
                    Err(err) => {
                        report.fail_case(format!("new failed: {err}"));
                        continue;
                    }
                };
                unflattener.single_cell_mode = *mode;
                match unflattener.unflatten(table) {
                    Ok(()) => match (expected, expect_error) {
                        (Some(expected), None) => {
                            if !are_equal(&dest, expected) {
                                report.fail_case(format!(
                                    "dest: expected {expected:?}, got {dest:?}"
                                ));
                            }
                        }
                        (_, Some(needle)) => report
                            .fail_case(format!("expected error containing {needle:?}, got Ok")),
                        (None, None) => report.fail_case("expected an error but got Ok"),
                    },
                    Err(err) => match expect_error {
                        Some(needle) => {
                            if !err.to_string().contains(needle) {
                                report.fail_case(format!(
                                    "error expected to contain {needle:?}, got {err}"
                                ));
                            }
                        }
                        None => report.fail_case(format!("unexpected error: {err}")),
                    },
                }
            }
        }
    }
    report.finish();
}

/// End-to-end: flatten a Vec<EmployeeRow>, then unflatten the table back and
/// expect the original collection.
#[test]
fn flatten_unflatten_roundtrip_case() {
    let mut report = TestReport::new(
        "flatten_unflatten_roundtrip_case",
        "flatten then unflatten reproduces the source collection (src/flattener + src/unflattener)",
    );
    report.begin_case("cartesian employees round-trip");

    let model = employee_model();
    let source = vec![
        EmployeeRow {
            id: vec![100],
            skills: vec!["Go".to_string()],
            profile: vec![ProfileRow {
                name: vec!["Dev".to_string()],
                age: vec![30],
                address: vec![
                    AddressRow {
                        street: vec!["Home St".to_string()],
                        city: vec!["Nairobi".to_string()],
                        zip: vec!["00100".to_string()],
                    },
                    AddressRow {
                        street: vec!["Work Ave".to_string()],
                        city: vec!["Westlands".to_string()],
                        zip: vec!["00200".to_string()],
                    },
                ],
            }],
        },
        EmployeeRow {
            id: vec![200],
            skills: vec!["Management".to_string()],
            profile: vec![ProfileRow {
                name: vec!["Admin".to_string()],
                age: vec![40],
                address: vec![AddressRow {
                    street: vec!["Office Rd".to_string()],
                    city: vec!["Mombasa".to_string()],
                    zip: vec!["80100".to_string()],
                }],
            }],
        },
    ];

    let mut flattener = match Flattener::new(&model) {
        Ok(f) => f,
        Err(err) => {
            report.fail_case(format!("flattener new failed: {err}"));
            report.finish();
            return;
        }
    };
    if let Err(err) = flattener.flatten(&source) {
        report.fail_case(format!("flatten failed: {err}"));
        report.finish();
        return;
    }
    let table = flattener.result().clone();

    let schema = Schema::Node(employee_schema());
    let mut dest: Vec<EmployeeRow> = Vec::new();
    let mut unflattener = match Unflattener::new(&model, &mut dest, Some(&schema)) {
        Ok(u) => u,
        Err(err) => {
            report.fail_case(format!("unflattener new failed: {err}"));
            report.finish();
            return;
        }
    };
    match unflattener.unflatten(&table) {
        Ok(()) => {
            if !are_equal(&dest, &source) {
                report.fail_case(format!("round-trip: expected {source:?}, got {dest:?}"));
            }
        }
        Err(err) => report.fail_case(format!("unflatten failed: {err}")),
    }
    report.finish();
}
