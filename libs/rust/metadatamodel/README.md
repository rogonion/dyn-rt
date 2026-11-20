# dyn-rt-metadatamodel

The metadata model as an extensible JSON-like `Value` tree on top of the dynamic layer, with subdomains: `core` (model vocabulary, `<section>:<key>` props, path-template post-processing `data_path`/`schema_path`, cardinality single/array), `fieldcolumns`, `database`, `iter`, `filter` (query conditions), `flattener` / `unflattener` (nested data <-> 2D table), `groupindex`. Facade: `dyn_rt::metadatamodel`.

```rust
use dyn_rt::json::Value;
use dyn_rt::metadatamodel::core;

let model = Value::object([
    (core::props::GROUP_PATH_KEY, Value::from("$")),
    (
        core::FIELDS_KEY,
        Value::from(vec![Value::object([(
            "ID",
            Value::from(vec![Value::object([(
                core::props::GROUP_PATH_KEY,
                Value::from("$['group:fields'][*].ID[*]"),
            )])]),
        )])]),
    ),
]);

// template -> actual path to data:
let path = core::data_path("$['group:fields'][*].ID[*]", &[], core::Cardinality::Array).unwrap();
assert_eq!(path, "$.ID[0]");
```

Full examples (all subdomains, mixed single/array cardinality, flatten/unflatten roundtrip, filter query conditions): <https://github.com/rogonion/dyn-rt/blob/main/docs/examples/rust/metadatamodel.md> — plus the long-form model + query-condition JSON in `docs/architecture/descriptions/metadatamodel.md`.

Part of [dyn-rt](https://github.com/rogonion/dyn-rt). License: MIT (see LICENSE).
