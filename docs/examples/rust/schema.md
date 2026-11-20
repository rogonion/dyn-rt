# Examples — `schema` (Rust)

Module crate: `dyn-rt-schema` (`libs/rust/schema`), facade `dyn_rt::schema`.
Feature `schema` (in `default`); text deserialization adapters need feature
`deser`.

A `Schema` is either a single `SchemaNode` or a `SchemaSet` (alternatives,
go's `DynamicSchema`). Schemas describe reflected data structurally — leaf
nodes pin a concrete scalar type by `TypeId` (identity), structure nodes
describe sequences, maps, structs, enums, indirections.

## Scalar node: validate + convert

```rust
use dyn_rt::schema::{Schema, SchemaNode, convert, validate};

let schema = Schema::Node(SchemaNode::scalar::<i64>());

let outcome = validate(&42i64, &schema).unwrap();
println!("{:?}", outcome.valid_set_keys); // [] (no Set involved)
assert!(validate(&String::from("nope"), &schema).is_err());

let mut target = 0i64;
convert(&42i64, &schema, &mut target).unwrap();
assert_eq!(target, 42);
```

## Struct node over a derived type

```rust
use dyn_rt::reflect::ReflectStruct;
use dyn_rt::schema::{Schema, SchemaNode, validate};

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
struct User {
    #[reflect(rename = "ID")]
    id: Vec<i64>,
    #[reflect(rename = "Name")]
    name: Vec<String>,
}

let user_schema = Schema::Node(SchemaNode::struct_node::<User>(vec![
    ("ID", Schema::Node(SchemaNode::sequence_node::<Vec<i64>>(
        Schema::Node(SchemaNode::scalar::<i64>())))),
    ("Name", Schema::Node(SchemaNode::sequence_node::<Vec<String>>(
        Schema::Node(SchemaNode::scalar::<String>())))),
]));

let user = User { id: vec![1], name: vec!["Alice".to_string()] };
assert!(validate(&user, &user_schema).is_ok());
```

Node constructors: `scalar::<T>()`, `json_scalar("string"|"number"|...)`,
`any()`, `sequence_node::<Vec<T>>(elements)`, `map_node::<HashMap<K,V>>(key,
value)`, `struct_node::<T>(fields)`, `enum_node::<T>(variants)`,
`indirection_node::<Box<T>>(inner)`.

## Schema sets (polymorphic values)

```rust
use dyn_rt::schema::{Schema, SchemaSet, SchemaNode, validate};

let set = Schema::Set(SchemaSet {
    default_key: Some("int".to_string()),
    nodes: vec![
        ("int".to_string(), SchemaNode::scalar::<i64>()),
        ("text".to_string(), SchemaNode::json_scalar("string")),
    ],
});
let outcome = validate(&7i64, &set).unwrap();
println!("{:?}", outcome.valid_set_keys); // ["int"] — which alternative matched
```

## 2-step deserialization (feature `deser`)

Parse JSON/YAML/TOML text into a reflect-able value (serde `Value` under the
hood, via the serde adapter), then convert it into a typed struct with a
schema:

```rust
use dyn_rt::reflect::{Reflect, ReflectStruct};
use dyn_rt::schema::{Schema, SchemaNode, convert, from_json_str, from_yaml_str, from_toml_str};

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
struct User {
    #[reflect(rename = "ID")]
    id: i64,
    #[reflect(rename = "Name")]
    name: String,
}

let schema = Schema::Node(SchemaNode::struct_node::<User>(vec![
    ("ID", Schema::Node(SchemaNode::scalar::<i64>())),
    ("Name", Schema::Node(SchemaNode::scalar::<String>())),
]));

let parsed = from_json_str(r#"{"ID": 1, "Name": "amy"}"#).unwrap(); // Box<dyn Reflect>
let mut user = User::default();
convert(parsed.as_ref(), &schema, &mut user).unwrap();
assert_eq!(user.id, 1);

// same with from_yaml_str("ID: 1\nName: amy\n") and from_toml_str(...)
```

A runnable version of this lives at
`packages/rust/dyn-rt/examples/schema_deser.rs`:
`cargo run -p dyn-rt --example schema_deser --features deser`.
