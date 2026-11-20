# Examples — `manipulation` (Rust)

Module crate: `dyn-rt-manipulation` (`libs/rust/manipulation`), facade
`dyn_rt::manipulation`. Feature `manipulation` (in `default`).

`DynObject` is the entrypoint: wrap a source (borrowed mutably, read-only, or
owned), then `get` / `set` / `delete` / `for_each` at a JSONPath given as a
string, a parsed `RecursiveDescentSegments`, or a single segment.

## DynObject wrappers

```rust
use dyn_rt::json::Value;
use dyn_rt::manipulation::DynObject;

let mut doc = Value::object([("ID", Value::from(vec![Value::from(101i64)]))]);

let mut obj = DynObject::new(&mut doc);        // borrowed mutable
let ro = DynObject::read_only(&doc);           // shared; set/delete fail
let owned = DynObject::owned(Box::new(doc));   // no external owner needed
```

## get — wildcards, negative indexes, unions, recursive descent

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

let obj = DynObject::read_only(&doc);
assert_eq!(obj.get("$.ID[*]").unwrap().len(), 2);      // wildcard
assert_eq!(obj.get("$.ID[-1]").unwrap().len(), 1);     // negative index
assert_eq!(obj.get("$.ID[0,1]").unwrap().len(), 2);    // union
assert_eq!(obj.get("$..City[*]").unwrap().len(), 2);   // recursive descent
```

`get` returns every match: `Vec<&dyn Reflect>`.

## set / delete

```rust
let mut obj = DynObject::new(&mut doc);
let n = obj.set("$.ID[0]", &Value::from(7i64)).unwrap(); // 1
let n = obj.delete("$.ID[1]").unwrap();                  // 1
// doc is now {"ID": [7], "Address": [...]}
```

Attach a schema to guide coercion/creation on `set`:
`DynObject::new(&mut doc).with_schema(&schema)` (or per-call
`set_with_schema(path, value, Some(&schema))`).

## for_each

```rust
let mut count = 0;
obj.for_each("$..City[*]", &mut |_v| { count += 1; true }).unwrap();
assert_eq!(count, 2); // returning false terminates the walk
```

## Deep equality

```rust
use dyn_rt::manipulation::are_equal;
use dyn_rt::json::Value;

assert!(are_equal(&Value::from(1i64), &Value::from(1i64)));
```

## Complex native types

Works on any reflected type, not just `Value` — including structs with
`Box`/`Option`/`HashMap`/tuple fields; see
[reflect.md](reflect.md#complex-nested-shapes-smart-pointers-maps-tuples) for a
full nested-pointer example (`$.Meta['k1'][0][1]`, `$.Maybe.City[0]`, ...).

## Runnable tour

`packages/rust/dyn-rt/examples/modules.rs` exercises every module:

```sh
cargo run -p dyn-rt --example modules --features metadatamodel
```
