# Examples — `shared` (Rust)

Module crate: `dyn-rt-shared` (`libs/rust/shared`). In the published facade it
is exposed as **`dyn_rt::json`** — `json` names only the JSON data model
(`Value` / `Number` / `JsonError`), not the dynamic layer (ADR 0010).

Enable: feature `json` (in `default`).

`Value` is the dependency-free JSON value model. Objects are ordered
key/value pairs (`Vec<(String, Value)>`), not a hash map, so key order is
preserved. `Value` itself implements `Reflect`, so it participates in the
whole dynamic layer (path, schema, manipulation, metadatamodel).

## Constructing values

```rust
use dyn_rt::json::Value;

let v = Value::object([
    ("name", Value::from("dyn-rt")),
    ("tags", Value::from(vec![Value::from("a"), Value::from("b")])),
    ("n", Value::from(42i64)),
]);

assert_eq!(v.get("name").and_then(|x| x.as_str()), Some("dyn-rt"));
assert_eq!(v.get("tags").and_then(|x| x.as_array()).map(|a| a.len()), Some(2));
```

`Value::from` covers `bool`, all integer/float types, `&str`, `String`, and
`Vec<Value>`. `Value::object(entries)` builds an object from any
`IntoIterator<Item = (K, V)>` with `K: Into<String>, V: Into<Value>`,
replacing duplicate keys.

## Reading values

```rust
let n = Value::from(42i64);
assert_eq!(n.as_number().map(|num| num.as_f64()), Some(42.0));
assert!(Value::Null.is_null());

let v = Value::object([("a", Value::from(1i64))]);
// mutable access too: v.get_mut("a"), v.as_object_mut(), v.as_array_mut()
```

`Number` distinguishes integer vs float (`is_integer`, `as_f64`).

## As a reflected value

Because `Value: Reflect`, it works anywhere a `&dyn Reflect` is accepted —
e.g. with `DynObject`:

```rust
use dyn_rt::json::Value;
use dyn_rt::manipulation::DynObject;

let mut doc = Value::object([("ID", Value::from(vec![Value::from(101i64)]))]);
let mut obj = DynObject::new(&mut doc);
let found = obj.get("$.ID[0]").unwrap(); // 1 value
```

See [manipulation.md](manipulation.md) for the full tour.
