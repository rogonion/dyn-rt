# dyn-rt-reflect (+ dyn-rt-reflect-derive)

General-purpose runtime reflection over Rust's own types — the go-`reflect` analogue. `Reflect` is the root trait; shape traits (`ReflectStruct`, `ReflectEnum`, `ReflectSequence`, `ReflectMap`, `ReflectTuple`, `ReflectIndirection`, `ReflectScalar`) are implemented for all primitives, `String`, `Vec`/slices, `HashMap`/`BTreeMap`, `Box<T>`, `Option<T>`, tuples up to 16, and `shared::Value`. Derive macros (`ReflectStruct`, `ReflectEnum`, `Reflect`) for user types. Usable independently; facade: `dyn_rt::reflect`.

```rust
use dyn_rt::reflect::{ReflectStruct, kind_of};

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
struct User {
    #[reflect(rename = "ID")]
    id: Vec<i64>,
    #[reflect(skip)]
    session_token: String,
}

let user = User { id: vec![1], session_token: "s".into() };
assert_eq!(kind_of(&user), dyn_rt::reflect::ReflectKind::Struct);
assert!(user.reflect_zero().is_some()); // #[reflect(new)]
```

Full examples (incl. enums with struct variants, `Box`/`Option`/`HashMap`/tuple nesting): <https://github.com/rogonion/dyn-rt/blob/main/docs/examples/rust/reflect.md>

Part of [dyn-rt](https://github.com/rogonion/dyn-rt). License: MIT (see LICENSE).
