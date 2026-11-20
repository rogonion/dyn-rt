# Examples — `reflect` (Rust)

Module crate: `dyn-rt-reflect` + `dyn-rt-reflect-derive` (`libs/rust/reflect`),
facade `dyn_rt::reflect`. Feature `reflect` (in `default`). General-purpose
runtime reflection — usable on its own, independent of the other modules.

## Kind and children of any value

```rust
use dyn_rt::reflect::{children, kind_of};

println!("{:?}", kind_of(&vec![1i64, 2, 3])); // Sequence
assert_eq!(children(&vec![1i64, 2, 3]).len(), 3);
```

`kind_of` returns a `ReflectKind`: `Scalar`, `Sequence`, `Map`, `Struct`,
`Enum`, `Tuple`, `Indirection`, `Unknown`. Built-in adapters cover all
primitive scalars, `String`, `()`, `Vec`/slices, `HashMap`/`BTreeMap`,
`Box<T>`, `Option<T>`, tuples up to 16, and `shared::Value`.

## Deriving reflection for a struct

```rust
use dyn_rt::reflect::{ReflectStruct, as_struct, kind_of};

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)] // enables reflect_zero(): a fresh default instance
struct User {
    #[reflect(rename = "ID")]   // exposed under this name (e.g. in paths/schemas)
    id: Vec<i64>,
    #[reflect(rename = "Name")]
    name: Vec<String>,
    #[reflect(skip)]           // not visible to reflection
    session_token: String,
}

let user = User { id: vec![1], name: vec!["Alice".to_string()], session_token: "s".into() };
assert_eq!(kind_of(&user), dyn_rt::reflect::ReflectKind::Struct);

let s = as_struct(&user).unwrap();
assert_eq!(s.reflect_field_infos().len(), 2);           // `session_token` is skipped
println!("{:?}", s.reflect_field("Name").map(kind_of)); // Some(Sequence)
```

Derive attributes: `#[reflect(name = "...")]` (display label only — never
compare names; identity is `TypeId`), `#[reflect(new)]`, field-level
`rename` / `skip`.

## Deriving reflection for an enum

```rust
use dyn_rt::reflect::{ReflectEnum, kind_of};

#[derive(ReflectEnum, Clone, Debug, PartialEq)]
enum Shape {
    Circle(f64),
    #[reflect(rename = "square")]
    Square { side: f64 }, // struct variant: named inline fields
    Empty,
}

let shape = Shape::Square { side: 2.0 };
assert_eq!(kind_of(&shape), dyn_rt::reflect::ReflectKind::Enum);
```

`Reflect` alone (no shape trait) marks a type as participating with kind
`Unknown` — for types you never intend to introspect.

## Clone, zero, downcast

```rust
use dyn_rt::reflect::{Reflect, kind_of};

let user = /* ... */;
let cloned = user.reflect_clone();          // deep clone, Box<dyn Reflect>
assert_eq!(kind_of(cloned.as_ref()), kind_of(&user));
let zero = user.reflect_zero();             // Some when #[reflect(new)]
assert!(zero.is_some());
let any = user.reflect_as_any();            // &dyn Any
assert!(any.downcast_ref::<User>().is_some());
```

## Complex nested shapes (smart pointers, maps, tuples)

Reflection and manipulation transparently see through `Box`/`Option`:

```rust
use std::collections::HashMap;
use dyn_rt::manipulation::DynObject;
use dyn_rt::reflect::{ReflectStruct, kind_of};

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
struct Address {
    #[reflect(rename = "City")]
    city: Vec<String>,
}

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
struct Complex {
    #[reflect(rename = "Meta")]
    meta: Box<Option<HashMap<String, Vec<(i64, String)>>>>,
    #[reflect(rename = "Maybe")]
    maybe: Option<Box<Address>>,
}

let mut c = Complex {
    meta: Box::new(Some(HashMap::from([(
        "k1".to_string(),
        vec![(1i64, "one".to_string()), (2, "two".to_string())],
    )]))),
    maybe: Some(Box::new(Address { city: vec!["Nairobi".to_string()] })),
};

let mut obj = DynObject::new(&mut c);
// map key, then sequence index, then tuple element:
let tup = obj.get("$.Meta['k1'][0][1]").unwrap();   // 1 value (the String "one")
// through Option<Box<Address>>:
let city = obj.get("$.Maybe.City[0]").unwrap();     // 1 value
let n = obj.set("$.Maybe.City[0]", &String::from("Mombasa")).unwrap();
assert_eq!(n, 1);
// c.maybe is now Some(Address { city: ["Mombasa"] })
```
