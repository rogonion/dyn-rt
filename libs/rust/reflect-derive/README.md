# dyn-rt-reflect-derive

Derive macros for `dyn-rt-reflect`: `#[derive(Reflect)]`, `#[derive(ReflectStruct)]`, `#[derive(ReflectEnum)]` with `#[reflect(new)]`, `#[reflect(name = "...")]`, field-level `#[reflect(rename = "...")]` / `#[reflect(skip)]`. Built on the de-facto proc-macro stack (syn/quote/proc-macro2), sanctioned by ADR 0005.

```rust
use dyn_rt::reflect::{ReflectStruct, kind_of};

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
struct User {
    #[reflect(rename = "ID")]
    id: Vec<i64>,
}

assert_eq!(kind_of(&User { id: vec![1] }), dyn_rt::reflect::ReflectKind::Struct);
```

Full examples: <https://github.com/rogonion/dyn-rt/blob/main/docs/examples/rust/reflect.md>

Part of [dyn-rt](https://github.com/rogonion/dyn-rt). License: MIT (see LICENSE).
