//! Per-module usage tour of the `dyn-rt` facade crate: json, path, reflect,
//! schema, manipulation, metadatamodel.
//!
//! Run: cargo run -p dyn-rt --example modules --features metadatamodel

use dyn_rt::json::Value;
use dyn_rt::manipulation::DynObject;
use dyn_rt::metadatamodel::core;
use dyn_rt::metadatamodel::flattener::Flattener;
use dyn_rt::reflect::{ReflectStruct, kind_of};
use dyn_rt::schema::{Schema, SchemaNode, convert, validate};

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
struct User {
    #[reflect(rename = "ID")]
    id: Vec<i64>,
    #[reflect(rename = "Name")]
    name: Vec<String>,
}

fn main() {
    // -- json: the JSON data model ------------------------------------------
    let mut doc = Value::object([
        ("ID", Value::from(vec![Value::from(101i64)])),
        ("Name", Value::from(vec![Value::from("Alice")])),
    ]);

    // -- path: parse a JSONPath into segments (Display renders it back) ----
    let segments = dyn_rt::path::parse("$.ID[0]");
    println!("path: {segments}");

    // -- reflect: runtime reflection over a native struct -------------------
    let user = User {
        id: vec![1],
        name: vec!["Alice".to_string()],
    };
    println!("reflect kind: {:?}", kind_of(&user));

    // -- schema: validate + convert -----------------------------------------
    let schema = Schema::Node(SchemaNode::scalar::<i64>());
    let outcome = validate(&42i64, &schema).unwrap();
    println!(
        "schema validate valid_set_keys: {:?}",
        outcome.valid_set_keys
    );
    let mut target = 0i64;
    convert(&42i64, &schema, &mut target).unwrap();
    println!("schema convert target: {target}");

    // -- manipulation: DynObject get / set / delete at a JSONPath -----------
    {
        let mut obj = DynObject::new(&mut doc);
        let found = obj.get("$.ID[0]").unwrap();
        println!("manipulation get count: {}", found.len());
        obj.set("$.Name[0]", &Value::from("Alicia")).unwrap();
        obj.delete("$.ID").unwrap();
    }
    println!("manipulation after set/delete: {doc:?}");

    // -- metadatamodel: templated group:path_key + flatten to a 2D table ----
    let model = Value::object([
        (core::props::GROUP_PATH_KEY, Value::from("$")),
        (
            core::FIELDS_KEY,
            Value::from(vec![Value::object([(
                "ID",
                Value::from(vec![Value::object([
                    (
                        core::props::GROUP_PATH_KEY,
                        Value::from("$['group:fields'][*].ID[*]"),
                    ),
                    (core::props::GROUP_IS_PRIMARY_KEY, Value::from(true)),
                ])]),
            )])]),
        ),
    ]);
    let data_path =
        core::data_path("$['group:fields'][*].ID[*]", &[], core::Cardinality::Array).unwrap();
    println!("metadatamodel data_path: {data_path}");

    let mut flattener = Flattener::new(&model).unwrap();
    flattener.flatten(&DynObject::read_only(&user)).unwrap();
    println!("metadatamodel table: {:?}", flattener.result());
}
