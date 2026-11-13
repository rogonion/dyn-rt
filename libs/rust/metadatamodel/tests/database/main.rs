//! Data-driven suites for the metadatamodel `database` subdomain
//! (ADR 0004/0009): src/database/{domain,application}.rs — extraction of a
//! table collection's columns (`get_column_fields`, port of go's
//! GetColumnFields) and get/set/delete of field values in source data
//! through those columns (`FieldValue`, port of go's FieldValue).

use dyn_rt_json_manipulation::application::are_equal;
use dyn_rt_json_schema::{Schema, SchemaNode};
use dyn_rt_metadatamodel::core::domain::{FIELDS_KEY, props};
use dyn_rt_metadatamodel::database::{FieldValue, GetColumnFieldsOptions, get_column_fields};
use dyn_rt_metadatamodel::fieldcolumns::PropMatch;
use dyn_rt_reflect::{Reflect, ReflectStruct};
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestReport, new_test_title};

/// Renders a reflected value for failure messages (Value/scalars; mirrors
/// json-manipulation's tests/common render).
fn render(r: &dyn Reflect) -> String {
    if let Some(v) = r.reflect_as_any().downcast_ref::<Value>() {
        return format!("{v:?}");
    }
    if let Some(b) = r.reflect_scalar_bool() {
        return format!("{b}");
    }
    if let Some(i) = r.reflect_scalar_i64() {
        return format!("{i}");
    }
    if let Some(f) = r.reflect_scalar_f64() {
        return format!("{f}");
    }
    if let Some(s) = r.reflect_scalar_string() {
        return format!("{s:?}");
    }
    format!("<{}>", r.reflect_type_name())
}

// ---------------------------------------------------------------------------
// sample models (port of go-metadatamodel/testdata Product/Employee models,
// with the section:key property names and fields-as-arrays)
// ---------------------------------------------------------------------------

/// A field entry: array whose first element is the props object, carrying
/// the db:* properties like go's testdata models.
fn db_field(
    path_key: &str,
    name: &str,
    column: &str,
    uid: &str,
    depth: i64,
    table: &str,
    primary: bool,
) -> Value {
    let mut entries: Vec<(&str, Value)> = vec![
        (props::GROUP_PATH_KEY, Value::from(path_key)),
        (props::GROUP_NAME, Value::from(name)),
        (props::DB_TABLE_COLLECTION_UID, Value::from(uid)),
        (props::DB_JOIN_DEPTH, Value::from(depth)),
        (props::DB_TABLE_COLLECTION_NAME, Value::from(table)),
        (props::DB_COLUMN_NAME, Value::from(column)),
    ];
    if primary {
        entries.push((props::GROUP_IS_PRIMARY_KEY, Value::from(true)));
    }
    Value::from(vec![Value::object(entries)])
}

/// A group entry (array with props object) holding nested fields.
fn db_group(
    path_key: &str,
    name: &str,
    uid: &str,
    depth: i64,
    table: &str,
    fields: Vec<(&str, Value)>,
) -> Value {
    Value::from(vec![Value::object([
        (props::GROUP_PATH_KEY, Value::from(path_key)),
        (props::GROUP_NAME, Value::from(name)),
        (props::DB_TABLE_COLLECTION_UID, Value::from(uid)),
        (props::DB_JOIN_DEPTH, Value::from(depth)),
        (props::DB_TABLE_COLLECTION_NAME, Value::from(table)),
        (FIELDS_KEY, Value::from(vec![Value::object(fields)])),
    ])])
}

/// Root group "Product" (uid Product, join depth 0) with fields ID, Name,
/// Price — port of go's testdata.ProductMetadataModel.
fn product_model() -> Value {
    Value::object([
        (props::GROUP_PATH_KEY, Value::from("$")),
        (props::GROUP_NAME, Value::from("Product")),
        (props::DB_TABLE_COLLECTION_UID, Value::from("Product")),
        (props::DB_JOIN_DEPTH, Value::from(0i64)),
        (props::DB_TABLE_COLLECTION_NAME, Value::from("Product")),
        (
            FIELDS_KEY,
            Value::from(vec![Value::object([
                (
                    "ID",
                    db_field(
                        "$['group:fields'][*].ID[*]",
                        "ID",
                        "ID",
                        "Product",
                        0,
                        "Product",
                        true,
                    ),
                ),
                (
                    "Name",
                    db_field(
                        "$['group:fields'][*].Name[*]",
                        "Name",
                        "Name",
                        "Product",
                        0,
                        "Product",
                        false,
                    ),
                ),
                (
                    "Price",
                    db_field(
                        "$['group:fields'][*].Price[*]",
                        "Price",
                        "Price",
                        "Product",
                        0,
                        "Product",
                        false,
                    ),
                ),
            ])]),
        ),
    ])
}

/// Root group "Employee" (uid Employee, join depth 0) with field ID and a
/// nested group "Profile" (uid Profile, join depth 1) holding Age and Bio —
/// port of go's testdata.EmployeeMetadataModel.
fn employee_model() -> Value {
    Value::object([
        (props::GROUP_PATH_KEY, Value::from("$")),
        (props::GROUP_NAME, Value::from("Employee")),
        (props::DB_TABLE_COLLECTION_UID, Value::from("Employee")),
        (props::DB_JOIN_DEPTH, Value::from(0i64)),
        (props::DB_TABLE_COLLECTION_NAME, Value::from("Employee")),
        (
            FIELDS_KEY,
            Value::from(vec![Value::object([
                (
                    "ID",
                    db_field(
                        "$['group:fields'][*].ID[*]",
                        "ID",
                        "ID",
                        "Employee",
                        0,
                        "Employee",
                        true,
                    ),
                ),
                (
                    "Profile",
                    db_group(
                        "$['group:fields'][*].Profile[*]",
                        "Profile",
                        "Profile",
                        1,
                        "Profile",
                        vec![
                            (
                                "Age",
                                db_field(
                                    "$['group:fields'][*].Profile[*]['group:fields'][*].Age[*]",
                                    "Age",
                                    "Age",
                                    "Profile",
                                    1,
                                    "Profile",
                                    false,
                                ),
                            ),
                            (
                                "Bio",
                                db_field(
                                    "$['group:fields'][*].Profile[*]['group:fields'][*].Bio[*]",
                                    "Bio",
                                    "Bio",
                                    "Profile",
                                    1,
                                    "Profile",
                                    false,
                                ),
                            ),
                        ],
                    ),
                ),
            ])]),
        ),
    ])
}

/// Small model whose field has no db:column_name.
fn no_column_name_model() -> Value {
    Value::object([
        (props::GROUP_PATH_KEY, Value::from("$")),
        (props::GROUP_NAME, Value::from("Broken")),
        (props::DB_TABLE_COLLECTION_UID, Value::from("Broken")),
        (
            FIELDS_KEY,
            Value::from(vec![Value::object([(
                "A",
                Value::from(vec![Value::object([
                    (
                        props::GROUP_PATH_KEY,
                        Value::from("$['group:fields'][*].A[*]"),
                    ),
                    (props::GROUP_NAME, Value::from("A")),
                    (props::DB_TABLE_COLLECTION_UID, Value::from("Broken")),
                ])]),
            )])]),
        ),
    ])
}

/// Small model with two fields sharing one db:column_name.
fn duplicate_column_model() -> Value {
    Value::object([
        (props::GROUP_PATH_KEY, Value::from("$")),
        (props::GROUP_NAME, Value::from("Dup")),
        (props::DB_TABLE_COLLECTION_UID, Value::from("Dup")),
        (
            FIELDS_KEY,
            Value::from(vec![Value::object([
                (
                    "A",
                    db_field(
                        "$['group:fields'][*].A[*]",
                        "A",
                        "Dup",
                        "Dup",
                        0,
                        "Dup",
                        false,
                    ),
                ),
                (
                    "B",
                    db_field(
                        "$['group:fields'][*].B[*]",
                        "B",
                        "Dup",
                        "Dup",
                        0,
                        "Dup",
                        false,
                    ),
                ),
            ])]),
        ),
    ])
}

// ---------------------------------------------------------------------------
// get_column_fields cases
// ---------------------------------------------------------------------------

struct GetColumnFieldsCase {
    test_title: Option<String>,
    model: fn() -> Value,
    options: GetColumnFieldsOptions,
    /// expected column names in read order, each with the field's path_key
    expected: Option<Vec<(&'static str, &'static str)>>,
    expect_error: Option<&'static str>,
}

macro_rules! gcf_case {
    ($model:expr, $options:expr, expected = $expected:expr $(, test_title = $title:expr)? $(, expect_error = $err:expr)?) => {
        GetColumnFieldsCase {
            test_title: None $(.or(Some($title.to_string())))?,
            model: $model,
            options: $options,
            expected: Some($expected),
            expect_error: None $(.or(Some($err)))?,
        }
    };
    ($model:expr, $options:expr, expect_error = $err:expr $(, test_title = $title:expr)?) => {
        GetColumnFieldsCase {
            test_title: None $(.or(Some($title.to_string())))?,
            model: $model,
            options: $options,
            expected: None,
            expect_error: Some($err),
        }
    };
}

fn uid_options(uid: &str) -> GetColumnFieldsOptions {
    GetColumnFieldsOptions {
        table_collection_uid: Some(uid.to_string()),
        ..Default::default()
    }
}

fn depth_name_options(depth: i64, name: &str) -> GetColumnFieldsOptions {
    GetColumnFieldsOptions {
        join_depth: Some(depth),
        table_collection_name: Some(name.to_string()),
        ..Default::default()
    }
}

#[test]
fn get_column_fields_cases() {
    let test_data: Vec<GetColumnFieldsCase> = vec![
        gcf_case!(
            product_model,
            uid_options("Product"),
            expected = vec![
                ("ID", "$['group:fields'][*].ID[*]"),
                ("Name", "$['group:fields'][*].Name[*]"),
                ("Price", "$['group:fields'][*].Price[*]"),
            ],
            test_title = "Product model by table_collection_uid"
        ),
        gcf_case!(
            product_model,
            depth_name_options(0, "Product"),
            expected = vec![
                ("ID", "$['group:fields'][*].ID[*]"),
                ("Name", "$['group:fields'][*].Name[*]"),
                ("Price", "$['group:fields'][*].Price[*]"),
            ],
            test_title = "Product model by join_depth 0 + table_collection_name"
        ),
        gcf_case!(
            employee_model,
            uid_options("Employee"),
            expected = vec![("ID", "$['group:fields'][*].ID[*]"),],
            test_title =
                "Employee model by uid Employee collects only root fields (groups not collected)"
        ),
        gcf_case!(
            employee_model,
            uid_options("Profile"),
            expected = vec![
                (
                    "Age",
                    "$['group:fields'][*].Profile[*]['group:fields'][*].Age[*]"
                ),
                (
                    "Bio",
                    "$['group:fields'][*].Profile[*]['group:fields'][*].Bio[*]"
                ),
            ],
            test_title = "Employee model by uid Profile collects nested group fields"
        ),
        gcf_case!(
            employee_model,
            depth_name_options(1, "Profile"),
            expected = vec![
                (
                    "Age",
                    "$['group:fields'][*].Profile[*]['group:fields'][*].Age[*]"
                ),
                (
                    "Bio",
                    "$['group:fields'][*].Profile[*]['group:fields'][*].Bio[*]"
                ),
            ],
            test_title = "Employee model by join_depth 1 + name Profile"
        ),
        gcf_case!(
            employee_model,
            GetColumnFieldsOptions {
                table_collection_uid: Some("Profile".to_string()),
                add: Some(PropMatch {
                    entries: vec![(props::DB_COLUMN_NAME.to_string(), Value::from("Age"))],
                }),
                ..Default::default()
            },
            expected = vec![(
                "Age",
                "$['group:fields'][*].Profile[*]['group:fields'][*].Age[*]"
            ),],
            test_title = "Employee model uid Profile with add db:column_name=Age collects only Age"
        ),
        gcf_case!(
            employee_model,
            GetColumnFieldsOptions {
                table_collection_uid: Some("Profile".to_string()),
                skip: Some(PropMatch {
                    entries: vec![(props::GROUP_NAME.to_string(), Value::from("Bio"))],
                }),
                ..Default::default()
            },
            expected = vec![(
                "Age",
                "$['group:fields'][*].Profile[*]['group:fields'][*].Age[*]"
            ),],
            test_title = "Employee model uid Profile with skip group:name=Bio skips Bio"
        ),
        gcf_case!(
            product_model,
            GetColumnFieldsOptions::default(),
            expect_error = "is required",
            test_title = "no uid and no join_depth+name is an error"
        ),
        gcf_case!(
            no_column_name_model,
            uid_options("Broken"),
            expect_error = "db:column_name not found",
            test_title = "field without db:column_name is an error"
        ),
        gcf_case!(
            duplicate_column_model,
            uid_options("Dup"),
            expect_error = "duplicate db:column_name",
            test_title = "duplicate db:column_name is an error"
        ),
    ];

    let mut report = TestReport::new(
        "get_column_fields_cases",
        "database get_column_fields (src/database/application.rs)",
    );
    for case in &test_data {
        report.begin_case(new_test_title!(case.test_title));
        let model = (case.model)();
        match get_column_fields(&model, &case.options) {
            Ok(fields) => match &case.expected {
                Some(expected) => {
                    let got_order: Vec<&str> =
                        fields.read_order.iter().map(|s| s.as_str()).collect();
                    let expected_order: Vec<&str> =
                        expected.iter().map(|(name, _)| *name).collect();
                    if got_order != expected_order {
                        report.fail_case(format!(
                            "read_order: expected {expected_order:?}, got {got_order:?}"
                        ));
                    }
                    for (name, path_key) in expected {
                        match fields.get(name) {
                            Some(props_value) => {
                                let got_key = props_value
                                    .get(props::GROUP_PATH_KEY)
                                    .and_then(Value::as_str)
                                    .unwrap_or("<missing>");
                                if got_key != *path_key {
                                    report.fail_case(format!(
                                        "column {name}: path_key expected {path_key:?}, got {got_key:?}"
                                    ));
                                }
                            }
                            None => report.fail_case(format!("column {name}: missing")),
                        }
                    }
                }
                None => report.fail_case("expected an error but got Ok"),
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

// ---------------------------------------------------------------------------
// field value cases (port of go's field_value_test.go, on native structs
// with Vec fields like go's testdata.Product/Employee)
// ---------------------------------------------------------------------------

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct ProductData {
    #[reflect(rename = "ID")]
    pub id: Vec<i64>,
    #[reflect(rename = "Name")]
    pub name: Vec<String>,
    #[reflect(rename = "Price")]
    pub price: Vec<f64>,
}

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct ProfileData {
    #[reflect(rename = "Age")]
    pub age: Vec<i64>,
    #[reflect(rename = "Bio")]
    pub bio: Vec<String>,
}

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct EmployeeData {
    #[reflect(rename = "ID")]
    pub id: Vec<i64>,
    #[reflect(rename = "Profile")]
    pub profile: Vec<ProfileData>,
}

fn int_seq_node() -> SchemaNode {
    SchemaNode::sequence_node::<Vec<i64>>(Schema::Node(SchemaNode::scalar::<i64>()))
}

fn string_seq_node() -> SchemaNode {
    SchemaNode::sequence_node::<Vec<String>>(Schema::Node(SchemaNode::scalar::<String>()))
}

fn float_seq_node() -> SchemaNode {
    SchemaNode::sequence_node::<Vec<f64>>(Schema::Node(SchemaNode::scalar::<f64>()))
}

fn product_data_schema() -> SchemaNode {
    SchemaNode::struct_node::<ProductData>(vec![
        ("ID", Schema::Node(int_seq_node())),
        ("Name", Schema::Node(string_seq_node())),
        ("Price", Schema::Node(float_seq_node())),
    ])
}

fn profile_data_schema() -> SchemaNode {
    SchemaNode::struct_node::<ProfileData>(vec![
        ("Age", Schema::Node(int_seq_node())),
        ("Bio", Schema::Node(string_seq_node())),
    ])
}

fn employee_data_schema() -> SchemaNode {
    SchemaNode::struct_node::<EmployeeData>(vec![
        ("ID", Schema::Node(int_seq_node())),
        (
            "Profile",
            Schema::Node(SchemaNode::sequence_node::<Vec<ProfileData>>(Schema::Node(
                profile_data_schema(),
            ))),
        ),
    ])
}

// source factories (each case gets a fresh source to mutate)
fn product_with_id() -> Box<dyn Reflect> {
    Box::new(ProductData {
        id: vec![1],
        ..Default::default()
    })
}

fn product_with_price() -> Box<dyn Reflect> {
    Box::new(ProductData {
        id: vec![1],
        price: vec![0.0],
        ..Default::default()
    })
}

fn product_with_twinkies() -> Box<dyn Reflect> {
    Box::new(ProductData {
        id: vec![1],
        name: vec!["Twinkies".to_string()],
        ..Default::default()
    })
}

fn product_price_deleted() -> Box<dyn Reflect> {
    Box::new(ProductData {
        id: vec![1],
        ..Default::default()
    })
}

fn employee_with_empty_profile() -> Box<dyn Reflect> {
    Box::new(EmployeeData {
        id: vec![1],
        profile: vec![ProfileData::default()],
    })
}

fn employee_with_age_16() -> Box<dyn Reflect> {
    Box::new(EmployeeData {
        id: vec![1],
        profile: vec![ProfileData {
            age: vec![16],
            ..Default::default()
        }],
    })
}

fn products_vec_with_id_7() -> Box<dyn Reflect> {
    Box::new(vec![ProductData {
        id: vec![7],
        ..Default::default()
    }])
}

fn name_meta_value() -> Box<dyn Reflect> {
    Box::new(Value::object([(
        "Name",
        Value::from(vec![Value::object([(
            "x",
            Value::object([("y", Value::from(5i64))]),
        )])]),
    )]))
}

enum FieldValueOp {
    /// get column, expect count + found value equal to expected
    Get {
        column: &'static str,
        suffix: &'static str,
        indices: Vec<usize>,
        expected_count: usize,
        expected: Box<dyn Reflect>,
    },
    /// set column to value, expect count + expected source afterwards
    Set {
        column: &'static str,
        value: Value,
        suffix: &'static str,
        indices: Vec<usize>,
        expected_count: usize,
        expected_source: fn() -> Box<dyn Reflect>,
    },
    /// delete column, expect count + expected source afterwards
    Delete {
        column: &'static str,
        suffix: &'static str,
        indices: Vec<usize>,
        expected_count: usize,
        expected_source: fn() -> Box<dyn Reflect>,
    },
    /// get with an unknown column name errors
    GetUnknownColumn { column: &'static str },
}

struct FieldValueCase {
    test_title: Option<String>,
    /// the metadata model to extract the columns from
    model: fn() -> Value,
    options: GetColumnFieldsOptions,
    /// fresh source data to manipulate
    source: fn() -> Box<dyn Reflect>,
    schema: Option<Schema>,
    op: FieldValueOp,
}

macro_rules! fv_case {
    ($model:expr, $options:expr, $source:expr, $schema:expr, $op:expr $(, test_title = $title:expr)?) => {
        FieldValueCase {
            test_title: None $(.or(Some($title.to_string())))?,
            model: $model,
            options: $options,
            source: $source,
            schema: $schema,
            op: $op,
        }
    };
}

#[test]
fn field_value_cases() {
    let test_data: Vec<FieldValueCase> = vec![
        // go TestDatabase_FieldValueOnProduct: Get ID
        fv_case!(
            product_model,
            depth_name_options(0, "Product"),
            product_with_id,
            Some(Schema::Node(product_data_schema())),
            FieldValueOp::Get {
                column: "ID",
                suffix: "",
                indices: vec![],
                expected_count: 1,
                expected: Box::new(1i64),
            },
            test_title = "get ID on Product returns the stored value (go FieldValueOnProduct Get)"
        ),
        // go TestDatabase_FieldValueOnProduct: Set Name
        fv_case!(
            product_model,
            depth_name_options(0, "Product"),
            product_with_id,
            Some(Schema::Node(product_data_schema())),
            FieldValueOp::Set {
                column: "Name",
                value: Value::from("Twinkies"),
                suffix: "",
                indices: vec![],
                expected_count: 1,
                expected_source: product_with_twinkies,
            },
            test_title = "set Name on Product inserts the value (go FieldValueOnProduct Set)"
        ),
        // go TestDatabase_FieldValueOnProduct: Delete Price
        fv_case!(
            product_model,
            depth_name_options(0, "Product"),
            product_with_price,
            Some(Schema::Node(product_data_schema())),
            FieldValueOp::Delete {
                column: "Price",
                suffix: "",
                indices: vec![],
                expected_count: 1,
                expected_source: product_price_deleted,
            },
            test_title =
                "delete Price on Product empties the value (go FieldValueOnProduct Delete)"
        ),
        // go TestDatabase_FieldValueOnEmployee: Set Age on the nested Profile
        fv_case!(
            employee_model,
            depth_name_options(1, "Profile"),
            employee_with_empty_profile,
            Some(Schema::Node(employee_data_schema())),
            FieldValueOp::Set {
                column: "Age",
                value: Value::from(16i64),
                suffix: "",
                indices: vec![],
                expected_count: 1,
                expected_source: employee_with_age_16,
            },
            test_title = "set Age on Employee's nested Profile (go FieldValueOnEmployee Set)"
        ),
        // source root is a sequence: first index addresses the root entry
        fv_case!(
            product_model,
            uid_options("Product"),
            products_vec_with_id_7,
            Some(Schema::Node(SchemaNode::sequence_node::<Vec<ProductData>>(
                Schema::Node(product_data_schema())
            ))),
            FieldValueOp::Get {
                column: "ID",
                suffix: "",
                indices: vec![0],
                expected_count: 1,
                expected: Box::new(7i64),
            },
            test_title = "get ID on a Vec<Product> source addresses the root entry first"
        ),
        // suffix path appended to the field's data path
        fv_case!(
            product_model,
            uid_options("Product"),
            name_meta_value,
            None,
            FieldValueOp::Get {
                column: "Name",
                suffix: "x.y",
                indices: vec![],
                expected_count: 1,
                expected: Box::new(Value::from(5i64)),
            },
            test_title = "get with a suffix path appends to the field's data path"
        ),
        fv_case!(
            product_model,
            uid_options("Product"),
            product_with_id,
            None,
            FieldValueOp::GetUnknownColumn { column: "Nope" },
            test_title = "get with an unknown column name is an error"
        ),
    ];

    let mut report = TestReport::new(
        "field_value_cases",
        "database FieldValue get/set/delete (src/database/application.rs)",
    );
    for case in &test_data {
        report.begin_case(new_test_title!(case.test_title));
        let model = (case.model)();
        let column_fields = match get_column_fields(&model, &case.options) {
            Ok(fields) => fields,
            Err(err) => {
                report.fail_case(format!("get_column_fields failed: {err}"));
                continue;
            }
        };
        let mut source = (case.source)();
        {
            let mut field_value = FieldValue::new(source.as_mut(), &column_fields);
            match &case.op {
                FieldValueOp::Get {
                    column,
                    suffix,
                    indices,
                    expected_count,
                    expected,
                } => match field_value.get(column, suffix, indices) {
                    Ok(found) => {
                        if found.len() != *expected_count {
                            report.fail_case(format!(
                                "count: expected {expected_count}, got {}",
                                found.len()
                            ));
                        } else if !found.is_empty() && !are_equal(found[0], expected.as_ref()) {
                            report.fail_case(format!(
                                "value: expected {}, got {}",
                                render(expected.as_ref()),
                                render(found[0])
                            ));
                        }
                    }
                    Err(err) => report.fail_case(format!("get failed: {err}")),
                },
                FieldValueOp::Set {
                    column,
                    value,
                    suffix,
                    indices,
                    expected_count,
                    ..
                } => match field_value.set(column, value, suffix, indices, case.schema.as_ref()) {
                    Ok(count) => {
                        if count != *expected_count {
                            report.fail_case(format!(
                                "count: expected {expected_count}, got {count}"
                            ));
                        }
                    }
                    Err(err) => report.fail_case(format!("set failed: {err}")),
                },
                FieldValueOp::Delete {
                    column,
                    suffix,
                    indices,
                    expected_count,
                    ..
                } => match field_value.delete(column, suffix, indices) {
                    Ok(count) => {
                        if count != *expected_count {
                            report.fail_case(format!(
                                "count: expected {expected_count}, got {count}"
                            ));
                        }
                    }
                    Err(err) => report.fail_case(format!("delete failed: {err}")),
                },
                FieldValueOp::GetUnknownColumn { column } => {
                    match field_value.get(column, "", &[]) {
                        Ok(_) => report.fail_case("expected an error but got Ok"),
                        Err(err) => {
                            if !err.to_string().contains("not found") {
                                report.fail_case(format!("unexpected error: {err}"));
                            }
                        }
                    }
                }
            }
        }
        // for set/delete, compare the mutated source with the expected source
        match &case.op {
            FieldValueOp::Set {
                expected_source, ..
            }
            | FieldValueOp::Delete {
                expected_source, ..
            } => {
                let expected = expected_source();
                if !are_equal(source.as_ref(), expected.as_ref()) {
                    report.fail_case(format!(
                        "source: expected {}, got {}",
                        render(expected.as_ref()),
                        render(source.as_ref())
                    ));
                }
            }
            FieldValueOp::Get { .. } | FieldValueOp::GetUnknownColumn { .. } => {}
        }
    }
    report.finish();
}
