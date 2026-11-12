# ADR 0008 — The reflect module (general-purpose runtime reflection)

- Status: accepted
- Date: 2026-11-11

## Context

Go has stdlib `reflect` and JS has native dynamic objects, so go-json/js-json
manipulate `any` directly. Rust has neither, and existing crates (bevy_reflect)
do not cover the intended design. dyn-rt-rs-old-3's `crates/reflection` is the
Rust guide: a `Reflect` gateway trait whose `reflect_ref()` returns the shape
handle directly, so kind and access are one fact.

## Decision

- `libs/rust/reflect` = crate `dyn-rt-reflect`, independent and general-purpose,
  porting the FULL dyn-rt-rs-old-3 reflection surface, not a trimmed subset —
  json-manipulation AND metadatamodel both consume it (metadatamodel needs
  `reflect_clone`, schema needs `TypeInfo`/`ReflectNew` construction):
  - `domain` (std only, Value-agnostic): `Reflect` trait (`reflect_type_name`,
    `reflect_ref`, `reflect_ref_mut`, `reflect_clone`, `reflect_apply`,
    `reflect_as_any`, `reflect_as_any_mut`, `reflect_into_any`, type-level
    `reflect_kind()`), `ReflectRef`/`ReflectRefMut` with all SEVEN variants
    (`Scalar(&dyn Reflect)`, `Sequence`, `Map`, `Struct`, `Enum`, `Indirection`,
    and `Unknown(&dyn Reflect)` — the base fallback: a type that implements
    only `Reflect` and no shape trait routes to `Unknown`, so uncovered types
    (e.g. `Cow`) are representable without forcing a `Reflect*` impl),
    `ReflectKind` (7 kinds), shape traits `ReflectSequence` / `ReflectMap` /
    `ReflectStruct` / `ReflectEnum` / `ReflectIndirection`, `ReflectNew` +
    `TypeInfo` + cached `type_info()`, `FieldInfo` / `VariantInfo`, and the
    full `ReflectError` vocabulary (`#[non_exhaustive]`).
  - `application`: thin semantic helpers (`kind_of`, per-kind `as_*` downcasts,
    `children`, `reflect_with_inner` / `reflect_with_inner_mut`).
  - Folder organization: `domain.rs` and `application.rs` stay flat single
    files (the whole trait system / all downcast helpers); only `adapters/`
    is split one file per `ReflectKind` (scalar, sequence, map, struct_shape,
    enum_shape, indirection, unknown, value), so the folder listing shows at
    a glance which built-in types are covered.
  - `adapters`: `impl Reflect` for `shared::Value` (Object→Map, Array→Sequence,
    scalars→Scalar; clone/apply/kind included), scalar primitives via a
    declarative macro (sanctioned by ADR 0005), `Vec<T>`→Sequence,
    `Box<T>`→Indirection, `Option<T>`→Enum, `Rc<T>`/`Arc<T>`/`RefCell<T>`/
    `Mutex<T>`→Indirection (share-clone for Rc/Arc, borrow-failure errors for
    guards). This is the only place the crate imports `shared`; later a
    `serde_json::Value` adapter lands here too (serde allowed in adapters only).
- Public APIs of dependent modules (json-manipulation, json-schema) take
  documents as `&dyn Reflect` — any type implementing `Reflect` is traversable;
  a value that does not implement a more concrete `Reflect*` shape simply
  reports `Scalar`/etc. and operations return `ReflectError::UnsupportedOperation`.
  Mis-implemented `Reflect*` contracts are the implementor's problem.
- Derive macros: a `libs/rust/reflect-derive` proc-macro crate using the
  de-facto helpers `syn` + `quote` will provide `#[derive(Reflect)]` for
  structs/enums (built when the schema module needs it; proc-macros must live
  in their own crate).
- `shared::Value` keeps objects as `Vec<(String, Value)>` (insertion order,
  std-only) — not `HashMap` (loses order) or `BTreeMap` (reorders). serde is a
  serialization library, not a domain model: the core stays std-only and
  serde_json conversion lives in adapters.

## Consequences

- reflect is usable standalone as a general-purpose reflection crate.
- json-manipulation resolves paths against `&dyn Reflect`, so it works on
  `Value` today and on serde/native adapters later without changes.
- Dependency graph: `shared` <- `reflect` <- `json-manipulation`/`json-schema`;
  `json-path` stays independent.
