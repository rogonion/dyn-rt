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

- `libs/rust/reflect` = crate `dyn-rt-reflect`, independent and general-purpose:
  - `domain` (std only, Value-agnostic): `Reflect` trait (`reflect_type_name`,
    `reflect_ref`, `reflect_ref_mut`), `ReflectRef`/`ReflectRefMut` enums
    carrying the shape handle (`Scalar` / `Sequence(&dyn ReflectSequence)` /
    `Map(&dyn ReflectMap)` / `Indirection(&dyn ReflectIndirection)`), `Kind`,
    and a closed `ReflectError` vocabulary. Deliberately trimmed from old-3:
    no `TypeInfo`/`ReflectNew` construction, no `ReflectEnum`/`ReflectStruct` —
    the JSON runtime needs kind + access + map/sequence/indirection only;
    schema-driven construction is the schema module's job later.
  - `application`: thin semantic helpers (`kind_of`, `as_map`/`as_sequence`
    (+ `_mut`), `children` for recursive descent).
  - `adapters`: `impl Reflect` for `shared::Value` (Object→Map, Array→Sequence,
    scalars→Scalar), scalar primitives via a declarative macro (sanctioned by
    ADR 0005), `Vec<T>`→Sequence, `Box<T>`→Indirection. This is the only place
    the crate imports `shared`; later a `serde_json::Value` adapter lands here
    too (serde allowed in adapters only).
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
