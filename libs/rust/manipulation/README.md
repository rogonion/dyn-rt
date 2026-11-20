# dyn-rt-manipulation

Dynamic manipulation of reflected data: `DynObject` (the entrypoint — borrowed-mut `new`, `read_only`, or `owned`) with `get` / `set` / `delete` / `for_each` at a JSONPath (string, parsed AST, or segment), plus `are_equal` deep comparison. Works on any `Reflect` type, not just `Value`. Facade: `dyn_rt::manipulation`.

```rust
use dyn_rt::json::Value;
use dyn_rt::manipulation::DynObject;

let mut doc = Value::object([
    ("ID", Value::from(vec![Value::from(101i64), Value::from(102i64)])),
    ("Address", Value::from(vec![Value::object([(
        "City",
        Value::from(vec![Value::from("Nairobi"), Value::from("Mombasa")]),
    )])])),
]);

let mut obj = DynObject::new(&mut doc);
assert_eq!(obj.get("$.ID[*]").unwrap().len(), 2);
assert_eq!(obj.get("$..City[*]").unwrap().len(), 2);
obj.set("$.ID[0]", &Value::from(7i64)).unwrap();
obj.delete("$.ID[1]").unwrap();
```

Full examples (negative indexes, unions, schema-guided set, for_each, complex pointer-nested structs): <https://github.com/rogonion/dyn-rt/blob/main/docs/examples/rust/manipulation.md>

Part of [dyn-rt](https://github.com/rogonion/dyn-rt). License: MIT (see LICENSE).
