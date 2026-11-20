# Examples — `metadatamodel` (Rust)

Module crate: `dyn-rt-metadatamodel` (`libs/rust/metadatamodel`), facade
`dyn_rt::metadatamodel`. Feature `metadatamodel`.

Subdomains: `core`, `fieldcolumns`, `database`, `iter`, `filter`,
`flattener`, `unflattener`, `groupindex`.

The metadata model is an extensible **JSON-like `Value` tree** (not a fixed
struct). Field/group properties use `<section>:<key>` names (`core::props`).
Structural keys are always arrays: `group:fields` is an array whose `[0]` is
the fields object; each field entry is an array whose `[0]` is the field's
props object. `group:path_key` templates keep their `[*]` placeholders
(e.g. `$['group:fields'][*].ID[*]`); `core::data_path` post-processes a
template into the actual path to data.

Cardinality: every field/group declares `group:cardinality` /
`field:cardinality` — `"array"` (default; every value is a list) or
`"single"` (every value is a single object/scalar — a committed choice,
violations are errors, never dynamic exceptions).

See also the long-form doc:
[docs/architecture/descriptions/metadatamodel.md](../../architecture/descriptions/metadatamodel.md)
for a comprehensive nested model (groups, `db:*` column mapping, separate
columns) and the full JSON structure of filter query conditions.

## The model (core vocabulary)

```rust
use dyn_rt::json::Value;
use dyn_rt::metadatamodel::core;

let model = Value::object([
    (core::props::GROUP_PATH_KEY, Value::from("$")),
    (
        core::FIELDS_KEY, // "group:fields"
        Value::from(vec![Value::object([
            (
                "ID",
                Value::from(vec![Value::object([
                    (core::props::GROUP_PATH_KEY, Value::from("$['group:fields'][*].ID[*]")),
                    (core::props::GROUP_IS_PRIMARY_KEY, Value::from(true)),
                    (core::props::DB_TABLE_COLLECTION_UID, Value::from("users")),
                    (core::props::DB_COLUMN_NAME, Value::from("id")),
                ])]),
            ),
            (
                "Name",
                Value::from(vec![Value::object([
                    (core::props::GROUP_PATH_KEY, Value::from("$['group:fields'][*].Name[*]")),
                    (core::props::DB_TABLE_COLLECTION_UID, Value::from("users")),
                    (core::props::DB_COLUMN_NAME, Value::from("name")),
                ])]),
            ),
        ])]),
    ),
]);

assert_eq!(core::path_key_of(&model).unwrap(), "$");
assert!(core::fields_of(&model).unwrap().as_object().is_some());
assert_eq!(core::cardinality_of(&model), core::Cardinality::Array); // default
```

## core — path templates: data_path / schema_path

```rust
use dyn_rt::metadatamodel::core;

// template -> actual path to DATA: structural group:fields segments are
// removed, each remaining [*] is replaced by the given indices (default 0)
let p = core::data_path("$['group:fields'][*].ID[*]", &[], core::Cardinality::Array).unwrap();
assert_eq!(p, "$.ID[0]");

// Single cardinality trims the trailing [*] instead of indexing it
let p = core::data_path("$['group:fields'][*].ID[*]", &[], core::Cardinality::Single).unwrap();
assert_eq!(p, "$.ID");

// template -> path to the field/group's SCHEMA (structure kept)
let p = core::schema_path("$['group:fields'][*].ID[*]", &[0]).unwrap();
assert_eq!(p, "$['group:fields'][0].ID[0]");
```

## fieldcolumns — extract the model's columns

```rust
use dyn_rt::metadatamodel::{core, fieldcolumns};

let columns = fieldcolumns::extract(&model, &Default::default()).unwrap();
assert_eq!(columns.current_read_order().len(), 2); // ID, Name
```

## database — column fields + FieldValue get/set/delete

```rust
use dyn_rt::manipulation::DynObject;
use dyn_rt::metadatamodel::{database, core};

let column_fields = database::get_column_fields(
    &model,
    &database::GetColumnFieldsOptions {
        table_collection_uid: Some("users".to_string()),
        ..Default::default()
    },
)
.unwrap();
assert_eq!(column_fields.read_order.len(), 2); // ["id", "name"]

// read/write values by db:column_name instead of by path:
let mut rows = /* Vec<Row> or Value matching the model */;
let mut fv = database::FieldValue::new(DynObject::new(&mut rows), &column_fields);
let values = fv.get("id", "", &[0]).unwrap();          // path resolved from the model
let n = fv.set("name", &dyn_rt::json::Value::from("Alicia"), "", &[0], None).unwrap();
```

## flattener / unflattener — nested data <-> 2D table

```rust
use dyn_rt::manipulation::{DynObject, are_equal};
use dyn_rt::metadatamodel::{flattener, unflattener};
use dyn_rt::reflect::ReflectStruct;
use dyn_rt::schema::{Schema, SchemaNode};

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
struct Row {
    #[reflect(rename = "ID")]
    id: Vec<i64>,
    #[reflect(rename = "Name")]
    name: Vec<String>,
}

let row_schema = Schema::Node(SchemaNode::sequence_node::<Vec<Row>>(Schema::Node(
    SchemaNode::struct_node::<Row>(vec![
        ("ID", Schema::Node(SchemaNode::sequence_node::<Vec<i64>>(Schema::Node(SchemaNode::scalar::<i64>())))),
        ("Name", Schema::Node(SchemaNode::sequence_node::<Vec<String>>(Schema::Node(SchemaNode::scalar::<String>())))),
    ]),
)));

let mut rows = vec![Row { id: vec![1], name: vec!["Alice".to_string()] }];

let mut fl = flattener::Flattener::new(&model).unwrap();
fl.flatten(&DynObject::new(&mut rows)).unwrap();
let table = fl.result().clone();
// table == [[ Array([Number(1)]), Array([String("Alice")]) ]]

// back: destination is a typed Vec with the schema attached (guides creation)
let mut dest: Vec<Row> = Vec::new();
let mut uf = unflattener::Unflattener::new(&model, DynObject::new(&mut dest).with_schema(&row_schema)).unwrap();
uf.unflatten(&table).unwrap();
assert!(are_equal(&dest, &rows));
```

### Mixed cardinality (array + single in one model)

A nested group with `group:cardinality: "single"` whose field is also
`"single"` — data values are plain objects/scalars, not lists:

```rust
use dyn_rt::json::Value;
use dyn_rt::metadatamodel::core;

let model = Value::object([
    (core::props::GROUP_PATH_KEY, Value::from("$")),
    (
        core::FIELDS_KEY,
        Value::from(vec![Value::object([
            ("ID", Value::from(vec![Value::object([(
                core::props::GROUP_PATH_KEY,
                Value::from("$['group:fields'][*].ID[*]"),
            )])])),
            ("Profile", Value::from(vec![Value::object([
                (core::props::GROUP_PATH_KEY, Value::from("$['group:fields'][*].Profile")),
                (core::props::GROUP_CARDINALITY, Value::from("single")),
                (
                    core::FIELDS_KEY,
                    Value::from(vec![Value::object([(
                        "Age",
                        Value::from(vec![Value::object([
                            (
                                core::props::GROUP_PATH_KEY,
                                Value::from("$['group:fields'][*].Profile['group:fields'][*].Age"),
                            ),
                            (core::props::FIELD_CARDINALITY, Value::from("single")),
                        ])]),
                    )])]),
                ),
            ])]),
        ])]),
    ),
]);

// Single trims the trailing [*]: "$.Profile.Age" (vs "$.ID[0]" for array)
let p = core::data_path(
    "$['group:fields'][*].Profile['group:fields'][*].Age",
    &[],
    core::Cardinality::Single,
).unwrap();
assert_eq!(p, "$.Profile.Age");
```

Flatten/unflatten handle mixed cardinality the same way — a `MixedRow { id:
Vec<i64>, profile: Profile }` with `profile: { age: 30 }` flattens to a
table and unflattens back to an equal value (verified roundtrip).

## filter — query conditions over model-described data

`DataFilter` runs a query-condition `Value` against every entry of the root
sequence and returns the indexes that did **not** pass. Condition shape:
`Type: "FieldGroup"` → `Value: { "<group:path_key template>": { "<Condition>":
{ "AssumedFieldType": ..., "Value": ... } } }`.

```rust
use dyn_rt::json::Value;
use dyn_rt::manipulation::DynObject;
use dyn_rt::metadatamodel::filter;

let query = Value::object([
    (filter::domain::query::TYPE, Value::from(filter::domain::query::TYPE_FIELD_GROUP)),
    (filter::domain::query::NEGATE, Value::from(false)),
    (
        filter::domain::query::VALUE,
        Value::object([(
            "$['group:fields'][*].Name[*]",
            Value::object([(
                filter::domain::condition::EQUAL_TO,
                Value::object([
                    (filter::domain::condition_props::ASSUMED_FIELD_TYPE,
                     Value::from(filter::domain::field_type::TEXT)),
                    (filter::domain::condition_props::VALUE, Value::from("Alice")),
                ]),
            )]),
        )]),
    ),
]);

let mut df = filter::DataFilter::new(DynObject::read_only(&rows), &model);
let rejected = df.filter(&query, "", "").unwrap();
// rows = [Alice, Bob]  ->  rejected == [1]  (indexes where Name != "Alice")
```

Conditions: `EqualTo`, `GreaterThan`, `LessThan`, `BeginsWith`, `EndsWith`,
`Contains`, `NoOfEntries*`; combined with logical operators (`And`/`Or`,
`Negate`). Full JSON examples: the metadatamodel description doc linked above.

## iter — walk / prune the model itself

```rust
use dyn_rt::metadatamodel::iter;

let mut keys = Vec::new();
iter::for_each(&model, &mut |props| {
    if let Some(k) = props.get(core::props::GROUP_PATH_KEY).and_then(|v| v.as_str()) {
        keys.push(k.to_string());
    }
    iter::Visit::default() // set terminate/skip_fields to control the walk
})
.unwrap();
// keys == ["$['group:fields'][*].ID[*]", "$['group:fields'][*].Name[*]"]
```

`iter::filter(&mut model, callback)` removes fields/groups in place;
`iter::map` transforms props.

## groupindex — stable instance indexes for a flattened table

```rust
use dyn_rt::json::Value;
use dyn_rt::metadatamodel::groupindex::{GroupIndexNode, Signature};

let sig = Signature::default(); // joins column values with '|'
let row = vec![Value::from(1i64), Value::from("Alice")];
let key = sig.generate_signature(&row, &[0]);
assert_eq!(key, "1");

let mut node = GroupIndexNode::default();
let collection = node.get_or_create_group("Profile");
let (index, _) = collection.get_or_create_instance(&key, "$");      // 0 (new)
let (again, _) = collection.get_or_create_instance(&key, "$");      // 0 (same instance)
```
