//! Data-driven suites for the metadatamodel `fieldcolumns` subdomain
//! (ADR 0004/0009): src/fieldcolumns/{domain,application}.rs — extraction
//! of model fields as flat 2D columns, skip, and reposition.

use dyn_rt_metadatamodel::core::domain::{FIELDS_KEY, props};
use dyn_rt_metadatamodel::fieldcolumns::{ColumnFields, ExtractOptions, PropMatch, extract};
use dyn_rt_shared::Value;
use dyn_rt_test_support::{TestReport, new_test_title};

// ---------------------------------------------------------------------------
// sample model
// ---------------------------------------------------------------------------

fn field(path_key: &str, name: &str, data_type: &str) -> Value {
    Value::from(vec![Value::object([
        (props::GROUP_PATH_KEY, Value::from(path_key)),
        (props::GROUP_NAME, Value::from(name)),
        (props::FIELD_DATA_TYPE, Value::from(data_type)),
    ])])
}

/// Root group with read order [SourceId, Bio, Skills, Site, Geo]:
/// - SourceId: plain field
/// - Bio: plain field with view:column_position before SourceId
/// - Skills: field with 2 separate columns (header format "skill [*]")
/// - Site: nested group with fields Country, City
/// - Geo: non-nested group expanded into 2 separate columns (Lat, Lng)
fn sample_model() -> Value {
    let source_id = field("$['group:fields'][*].SourceId[*]", "source id", "Number");
    let bio_props = field("$['group:fields'][*].Bio[*]", "bio", "Boolean")
        .as_array()
        .unwrap()[0]
        .clone();
    let mut bio = bio_props;
    if let Some(obj) = bio.as_object_mut() {
        obj.push((
            props::VIEW_COLUMN_POSITION.to_string(),
            Value::object([
                (
                    props::GROUP_PATH_KEY,
                    Value::from("$['group:fields'][*].SourceId[*]"),
                ),
                (props::VIEW_POSITION_BEFORE, Value::from(true)),
            ]),
        ));
    }
    let bio = Value::from(vec![bio]);

    let mut skills_inner = field("$['group:fields'][*].Skills[*]", "skill", "Text")
        .as_array()
        .unwrap()[0]
        .clone();
    if let Some(obj) = skills_inner.as_object_mut() {
        obj.push((
            props::VIEW_VALUES_IN_SEPARATE_COLUMNS.to_string(),
            Value::from(true),
        ));
        obj.push((
            props::VIEW_MAX_VALUES_IN_SEPARATE_COLUMNS.to_string(),
            Value::from(2i64),
        ));
        obj.push((
            props::VIEW_SEPARATE_COLUMNS_HEADER_FORMAT.to_string(),
            Value::from("skill [*]"),
        ));
    }
    let skills = Value::from(vec![skills_inner]);

    let site = Value::from(vec![Value::object([
        (
            props::GROUP_PATH_KEY,
            Value::from("$['group:fields'][*].Site[*]"),
        ),
        (props::GROUP_NAME, Value::from("site")),
        (
            FIELDS_KEY,
            Value::from(vec![Value::object([
                (
                    "Country",
                    field(
                        "$['group:fields'][*].Site[*]['group:fields'][*].Country[*]",
                        "country",
                        "Text",
                    ),
                ),
                (
                    "City",
                    field(
                        "$['group:fields'][*].Site[*]['group:fields'][*].City[*]",
                        "city",
                        "Text",
                    ),
                ),
            ])]),
        ),
    ])]);

    let mut geo_inner = Value::object([
        (
            props::GROUP_PATH_KEY,
            Value::from("$['group:fields'][*].Geo[*]"),
        ),
        (props::GROUP_NAME, Value::from("geo")),
        (props::VIEW_VALUES_IN_SEPARATE_COLUMNS, Value::from(true)),
        (
            props::VIEW_MAX_VALUES_IN_SEPARATE_COLUMNS,
            Value::from(2i64),
        ),
        (
            FIELDS_KEY,
            Value::from(vec![Value::object([
                (
                    "Lat",
                    field(
                        "$['group:fields'][*].Geo[*]['group:fields'][*].Lat[*]",
                        "lat",
                        "Number",
                    ),
                ),
                (
                    "Lng",
                    field(
                        "$['group:fields'][*].Geo[*]['group:fields'][*].Lng[*]",
                        "lng",
                        "Number",
                    ),
                ),
            ])]),
        ),
    ]);
    // mark one sub-field disabled for the skip case
    let geo = Value::from(vec![geo_inner.clone()]);
    let _ = &mut geo_inner;

    Value::object([
        (props::GROUP_PATH_KEY, Value::from("$")),
        (props::GROUP_NAME, Value::from("root")),
        (
            props::GROUP_READ_ORDER_OF_FIELDS,
            Value::from(vec![
                Value::from("SourceId"),
                Value::from("Bio"),
                Value::from("Skills"),
                Value::from("Site"),
                Value::from("Geo"),
            ]),
        ),
        (
            FIELDS_KEY,
            Value::from(vec![Value::object([
                ("SourceId", source_id),
                ("Bio", bio),
                ("Skills", skills),
                ("Site", site),
                ("Geo", geo),
            ])]),
        ),
    ])
}

// ---------------------------------------------------------------------------
// cases
// ---------------------------------------------------------------------------

enum FieldColumnsOp {
    /// extract and check column keys + names in read order
    Extract {
        expected_keys: Vec<&'static str>,
        expected_names: Vec<&'static str>,
    },
    /// extract, then skip columns whose props match, check remaining keys
    ExtractSkip {
        skip: PropMatch,
        expected_keys: Vec<&'static str>,
    },
    /// extract, then reposition, check the current order of column keys
    ExtractReposition { expected_keys: Vec<&'static str> },
}

struct FieldColumnsCase {
    test_title: Option<String>,
    op: FieldColumnsOp,
}

macro_rules! fc_case {
    ($op:expr $(, test_title = $title:expr)?) => {
        FieldColumnsCase {
            test_title: None $(.or(Some($title.to_string())))?,
            op: $op,
        }
    };
}

fn column_keys(fields: &ColumnFields, order: &[usize]) -> Vec<String> {
    order
        .iter()
        .map(|original| fields.read_order[*original].column_key())
        .collect()
}

fn column_names(fields: &ColumnFields, order: &[usize]) -> Vec<String> {
    order
        .iter()
        .map(|original| {
            fields
                .get_by_index_in_original_read_order(*original)
                .and_then(|f| f.props.get(props::GROUP_NAME))
                .and_then(Value::as_str)
                .unwrap_or("<missing>")
                .to_string()
        })
        .collect()
}

#[test]
fn fieldcolumns_cases() {
    let test_data: Vec<FieldColumnsCase> = vec![
        fc_case!(
            FieldColumnsOp::Extract {
                expected_keys: vec![
                    "$['group:fields'][*].SourceId[*]",
                    "$['group:fields'][*].Bio[*]",
                    "$['group:fields'][*].Skills[*][0]",
                    "$['group:fields'][*].Skills[*][1]",
                    "$['group:fields'][*].Site[*]['group:fields'][*].Country[*]",
                    "$['group:fields'][*].Site[*]['group:fields'][*].City[*]",
                    "$['group:fields'][*].Geo[*][0].Lat",
                    "$['group:fields'][*].Geo[*][0].Lng",
                    "$['group:fields'][*].Geo[*][1].Lat",
                    "$['group:fields'][*].Geo[*][1].Lng",
                ],
                expected_names: vec![
                    "source id",
                    "bio",
                    "skill 1",
                    "skill 2",
                    "country",
                    "city",
                    "lat 1",
                    "lng 1",
                    "lat 2",
                    "lng 2",
                ],
            },
            test_title =
                "extract: plain fields, separate-column field, nested group, group-as-columns"
        ),
        fc_case!(
            FieldColumnsOp::ExtractSkip {
                skip: PropMatch {
                    entries: vec![(props::GROUP_NAME.to_string(), Value::from("bio"),)],
                },
                expected_keys: vec![
                    "$['group:fields'][*].SourceId[*]",
                    "$['group:fields'][*].Skills[*][0]",
                    "$['group:fields'][*].Skills[*][1]",
                    "$['group:fields'][*].Site[*]['group:fields'][*].Country[*]",
                    "$['group:fields'][*].Site[*]['group:fields'][*].City[*]",
                    "$['group:fields'][*].Geo[*][0].Lat",
                    "$['group:fields'][*].Geo[*][0].Lng",
                    "$['group:fields'][*].Geo[*][1].Lat",
                    "$['group:fields'][*].Geo[*][1].Lng",
                ],
            },
            test_title = "skip removes matching columns from the current read order"
        ),
        fc_case!(
            FieldColumnsOp::ExtractReposition {
                expected_keys: vec![
                    "$['group:fields'][*].Bio[*]",
                    "$['group:fields'][*].SourceId[*]",
                    "$['group:fields'][*].Skills[*][0]",
                    "$['group:fields'][*].Skills[*][1]",
                    "$['group:fields'][*].Site[*]['group:fields'][*].Country[*]",
                    "$['group:fields'][*].Site[*]['group:fields'][*].City[*]",
                    "$['group:fields'][*].Geo[*][0].Lat",
                    "$['group:fields'][*].Geo[*][0].Lng",
                    "$['group:fields'][*].Geo[*][1].Lat",
                    "$['group:fields'][*].Geo[*][1].Lng",
                ],
            },
            test_title = "view:column_position with position_before moves bio before source id"
        ),
    ];

    let mut report = TestReport::new(
        "fieldcolumns_cases",
        "fieldcolumns extraction / skip / reposition (src/fieldcolumns)",
    );
    for case in &test_data {
        report.begin_case(new_test_title!(case.test_title));
        let model = sample_model();
        let options = ExtractOptions::default();
        let result = extract(&model, &options);
        let mut fields = match result {
            Ok(fields) => fields,
            Err(err) => {
                report.fail_case(format!("unexpected error: {err}"));
                continue;
            }
        };
        match &case.op {
            FieldColumnsOp::Extract {
                expected_keys,
                expected_names,
            } => {
                let keys = column_keys(&fields, &fields.current.clone());
                if keys
                    != expected_keys
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                {
                    report.fail_case(format!("keys: expected {expected_keys:?}, got {keys:?}"));
                }
                let names = column_names(&fields, &fields.current.clone());
                if names
                    != expected_names
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                {
                    report.fail_case(format!("names: expected {expected_names:?}, got {names:?}"));
                }
            }
            FieldColumnsOp::ExtractSkip {
                skip,
                expected_keys,
            } => {
                fields.skip(Some(skip), None);
                let order = fields.current_read_order();
                let keys = column_keys(&fields, &order);
                if keys
                    != expected_keys
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                {
                    report.fail_case(format!("keys: expected {expected_keys:?}, got {keys:?}"));
                }
            }
            FieldColumnsOp::ExtractReposition { expected_keys } => {
                fields.reposition();
                fields.update_index_in_repositioned();
                let keys = column_keys(&fields, &fields.current.clone());
                if keys
                    != expected_keys
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                {
                    report.fail_case(format!("keys: expected {expected_keys:?}, got {keys:?}"));
                }
            }
        }
    }
    report.finish();
}
