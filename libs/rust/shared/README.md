# dyn-rt-shared

The JSON data model for dyn-rt: `Value` / `Number` / `JsonError` — dependency-free, ordered-key objects. In the published facade crate this module is exposed as **`dyn_rt::json`** (ADR 0010: `json` names only the data model). `Value` implements `Reflect`, so it participates in the whole dynamic layer.

```rust
use dyn_rt::json::Value;

let v = Value::object([
    ("name", Value::from("dyn-rt")),
    ("tags", Value::from(vec![Value::from("a"), Value::from("b")])),
]);
assert_eq!(v.get("name").and_then(|x| x.as_str()), Some("dyn-rt"));
assert_eq!(v.get("tags").and_then(|x| x.as_array()).map(|a| a.len()), Some(2));
```

Full examples: <https://github.com/rogonion/dyn-rt/blob/main/docs/examples/rust/shared.md>

Part of [dyn-rt](https://github.com/rogonion/dyn-rt) — a dynamic runtime layer over a language's own data types (see the repo README and `docs/architecture/`).

License: MIT (see LICENSE).
