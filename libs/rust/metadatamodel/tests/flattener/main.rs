//! Data-driven suites for the metadatamodel `flattener` subdomain
//! (ADR 0004/0009): src/flattener/{domain,application}.rs — converting
//! nested objects/collections into a 2D table (port of
//! go-metadatamodel/flattener's flatten_test.go), incl. single-cardinality
//! fields/groups and WriteToDestination.

use dyn_rt_json_manipulation::application::are_equal;
use dyn_rt_metadatamodel::core::domain::{FIELDS_KEY, props};
use dyn_rt_metadatamodel::flattener::Flattener;
use dyn_rt_reflect::{Reflect, ReflectStruct};
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestReport, new_test_title};

// ---------------------------------------------------------------------------
// source data shapes (go's testdata.User/Product/Employee with slice fields)
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
pub struct ProductRow {
    #[reflect(rename = "ID")]
    pub id: Vec<i64>,
    #[reflect(rename = "Name")]
    pub name: Vec<String>,
    #[reflect(rename = "Price")]
    pub price: Vec<f64>,
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

/// Employee whose Profile group is a single object (group:cardinality single).
#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct EmployeeSingleProfile {
    #[reflect(rename = "ID")]
    pub id: Vec<i64>,
    #[reflect(rename = "Profile")]
    pub profile: ProfileRow,
}

/// User with a single-cardinality scalar field Bio.
#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct UserWithBio {
    #[reflect(rename = "ID")]
    pub id: Vec<i64>,
    #[reflect(rename = "Bio")]
    pub bio: bool,
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

fn grp_single(path_key: &str, name: &str, fields: Vec<(&str, Value)>) -> Value {
    Value::from(vec![Value::object([
        (props::GROUP_PATH_KEY, Value::from(path_key)),
        (props::GROUP_NAME, Value::from(name)),
        (props::GROUP_CARDINALITY, Value::from("single")),
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

fn product_model() -> Value {
    Value::object([
        (props::GROUP_PATH_KEY, Value::from("$")),
        (props::GROUP_NAME, Value::from("Product")),
        (
            FIELDS_KEY,
            Value::from(vec![Value::object(vec![
                (
                    "ID",
                    fld_primary("$['group:fields'][*].ID[*]", "ID", "Number"),
                ),
                ("Name", fld("$['group:fields'][*].Name[*]", "Name", "Text")),
                (
                    "Price",
                    fld("$['group:fields'][*].Price[*]", "Price", "Number"),
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

/// User model with a single-cardinality Bio field.
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

/// Employee model with a single-cardinality Profile group.
fn employee_single_profile_model() -> Value {
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
                    grp_single(
                        "$['group:fields'][*].Profile[*]",
                        "Profile",
                        vec![(
                            "Name",
                            fld(
                                "$['group:fields'][*].Profile[*]['group:fields'][*].Name[*]",
                                "Name",
                                "Text",
                            ),
                        )],
                    ),
                ),
            ])]),
        ),
    ])
}

// ---------------------------------------------------------------------------
// source data
// ---------------------------------------------------------------------------

fn user_alice() -> Box<dyn Reflect> {
    Box::new(UserRow {
        id: vec![101],
        name: vec!["Alice".to_string()],
        email: vec!["alice@example.com".to_string()],
    })
}

fn employee_bob() -> Box<dyn Reflect> {
    Box::new(EmployeeRow {
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
    })
}

fn products() -> Box<dyn Reflect> {
    Box::new(vec![
        ProductRow {
            id: vec![1],
            name: vec!["Laptop".to_string()],
            price: vec![999.99],
        },
        ProductRow {
            id: vec![2],
            name: vec!["Mouse".to_string()],
            price: vec![25.50],
        },
    ])
}

fn cartesian_employees() -> Box<dyn Reflect> {
    Box::new(vec![
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
    ])
}

fn user_with_bio() -> Box<dyn Reflect> {
    Box::new(UserWithBio {
        id: vec![7],
        bio: true,
    })
}

fn employee_single_profile() -> Box<dyn Reflect> {
    Box::new(EmployeeSingleProfile {
        id: vec![9],
        profile: ProfileRow {
            name: vec!["Solo".to_string()],
            ..Default::default()
        },
    })
}

// ---------------------------------------------------------------------------
// expected-table helpers
// ---------------------------------------------------------------------------

/// A cell is always an array (the flattener's array formality).
fn cell(items: Vec<Value>) -> Value {
    Value::from(items)
}

fn cells(values: Vec<i64>) -> Value {
    cell(values.into_iter().map(Value::from).collect())
}

fn str_cells(values: Vec<&str>) -> Value {
    cell(values.into_iter().map(Value::from).collect())
}

fn table(rows: Vec<Vec<Value>>) -> Value {
    Value::from(rows.into_iter().map(Value::from).collect::<Vec<_>>())
}

fn table_of(result: &dyn_rt_metadatamodel::flattener::FlattenedTable) -> Value {
    table(result.clone())
}

// ---------------------------------------------------------------------------
// cases
// ---------------------------------------------------------------------------

struct FlattenCase {
    test_title: Option<String>,
    model: fn() -> Value,
    source: fn() -> Box<dyn Reflect>,
    expected: Value,
}

macro_rules! flatten_case {
    ($model:expr, $source:expr, $expected:expr $(, test_title = $title:expr)?) => {
        FlattenCase {
            test_title: None $(.or(Some($title.to_string())))?,
            model: $model,
            source: $source,
            expected: $expected,
        }
    };
}

#[test]
fn flatten_cases() {
    let test_data: Vec<FlattenCase> = vec![
        // go case 1: simple flat user
        flatten_case!(
            user_model,
            user_alice,
            table(vec![vec![
                cells(vec![101]),
                str_cells(vec!["Alice"]),
                str_cells(vec!["alice@example.com"]),
            ]]),
            test_title = "simple user flattens to one row of array cells (go case 1)"
        ),
        // go case 2: deep nested employee
        flatten_case!(
            employee_model,
            employee_bob,
            table(vec![vec![
                cells(vec![500]),
                str_cells(vec!["Bob"]),
                cells(vec![30]),
                str_cells(vec!["123 Tech Ln"]),
                str_cells(vec!["Silicon Valley"]),
                str_cells(vec!["94000"]),
                str_cells(vec!["Go", "Rust"]),
            ]]),
            test_title = "deep nested employee flattens to one row, 7 columns (go case 2)"
        ),
        // go case 3: array of products
        flatten_case!(
            product_model,
            products,
            table(vec![
                vec![
                    cells(vec![1]),
                    str_cells(vec!["Laptop"]),
                    cell(vec![Value::from(999.99)])
                ],
                vec![
                    cells(vec![2]),
                    str_cells(vec!["Mouse"]),
                    cell(vec![Value::from(25.50)])
                ],
            ]),
            test_title = "array of products flattens to one row per product (go case 3)"
        ),
        // go case 5: cartesian product across profiles and addresses
        flatten_case!(
            employee_model,
            cartesian_employees,
            table(vec![
                vec![
                    cells(vec![100]),
                    str_cells(vec!["Dev"]),
                    cells(vec![30]),
                    str_cells(vec!["Home St"]),
                    str_cells(vec!["Nairobi"]),
                    str_cells(vec!["00100"]),
                    str_cells(vec!["Go"]),
                ],
                vec![
                    cells(vec![100]),
                    str_cells(vec!["Dev"]),
                    cells(vec![30]),
                    str_cells(vec!["Work Ave"]),
                    str_cells(vec!["Westlands"]),
                    str_cells(vec!["00200"]),
                    str_cells(vec!["Go"]),
                ],
                vec![
                    cells(vec![200]),
                    str_cells(vec!["Admin"]),
                    cells(vec![40]),
                    str_cells(vec!["Office Rd"]),
                    str_cells(vec!["Mombasa"]),
                    str_cells(vec!["80100"]),
                    str_cells(vec!["Management"]),
                ],
                vec![
                    cells(vec![200]),
                    str_cells(vec!["Consultant"]),
                    cells(vec![35]),
                    str_cells(vec!["Remote"]),
                    str_cells(vec!["Eldoret"]),
                    str_cells(vec!["30100"]),
                    str_cells(vec!["Management"]),
                ],
            ]),
            test_title = "multiple employees/profiles/addresses produce the cartesian product of 4 rows (go case 5)"
        ),
        // single-cardinality scalar field: cell wraps the scalar
        flatten_case!(
            user_bio_model,
            user_with_bio,
            table(vec![vec![cells(vec![7]), cell(vec![Value::from(true)])]]),
            test_title = "single-cardinality field reads the scalar and wraps it in a cell array"
        ),
        // single-cardinality group: no array branching, children read under the object
        flatten_case!(
            employee_single_profile_model,
            employee_single_profile,
            table(vec![vec![cells(vec![9]), str_cells(vec!["Solo"])]]),
            test_title = "single-cardinality group flattens without branching (group [*] dropped)"
        ),
    ];

    let mut report = TestReport::new(
        "flatten_cases",
        "flattener: objects/collections to 2D table (src/flattener)",
    );
    for case in &test_data {
        report.begin_case(new_test_title!(case.test_title));
        let model = (case.model)();
        let source = (case.source)();
        match Flattener::new(&model) {
            Ok(mut flattener) => match flattener.flatten(source.as_ref()) {
                Ok(()) => {
                    let got = table_of(flattener.result());
                    if !are_equal(&got, &case.expected) {
                        report.fail_case(format!(
                            "table: expected {:?}, got {:?}",
                            case.expected, got
                        ));
                    }
                }
                Err(err) => report.fail_case(format!("flatten failed: {err}")),
            },
            Err(err) => report.fail_case(format!("new failed: {err}")),
        }
    }
    report.finish();
}

#[test]
fn write_to_destination_case() {
    let mut report = TestReport::new(
        "write_to_destination_case",
        "flattener write_to_destination honors the unskipped column read order (src/flattener)",
    );
    report.begin_case("products table writes into a pre-sized 2D destination");

    let model = product_model();
    let source = products();
    let mut flattener = match Flattener::new(&model) {
        Ok(f) => f,
        Err(err) => {
            report.fail_case(format!("new failed: {err}"));
            report.finish();
            return;
        }
    };
    if let Err(err) = flattener.flatten(source.as_ref()) {
        report.fail_case(format!("flatten failed: {err}"));
        report.finish();
        return;
    }

    // pre-size the destination 2 rows x 3 cols
    let mut destination = Value::from(vec![
        Value::from(vec![Value::Null, Value::Null, Value::Null]),
        Value::from(vec![Value::Null, Value::Null, Value::Null]),
    ]);
    if let Err(err) = flattener.write_to_destination(&mut destination, None) {
        report.fail_case(format!("write_to_destination failed: {err}"));
    } else {
        let expected = table_of(flattener.result());
        if !are_equal(&destination, &expected) {
            report.fail_case(format!(
                "destination: expected {:?}, got {:?}",
                expected, destination
            ));
        }
    }
    report.finish();
}
