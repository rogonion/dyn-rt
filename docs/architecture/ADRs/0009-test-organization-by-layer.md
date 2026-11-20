# ADR 0009 — Test organization: tests mirror the hexagon layers

- Status: accepted
- Date: 2026-11-12

## Context

ADR 0004 fixed the _shape_ of tests (in-language data tables, custom loops,
`test_title` fields) but not _where_ they live. As modules grow, a flat
`tests/` dir mixes suites for domain, application and adapters, so it is not
obvious which file tests which layer.

## Decision

- Tests live in the module crate's own `tests/` dir and mirror the crate's
  `src/` layers (ADR 0002/0007):
  - `tests/domain.rs` — suites for `src/domain` (models, ports, rendering).
  - `tests/application.rs` — suites for `src/application` use cases.
  - `tests/adapters/main.rs` + `tests/adapters/<file>.rs` — one submodule per
    `src/adapters/` file (cargo treats `tests/adapters/main.rs` as the
    `adapters` test target; `#[path = "../common/mod.rs"]` shares helpers).
  - Single flat file vs subfolder is a length call: a layer with one suite is
    one file (`tests/application.rs`); a layer with several files gets a
    folder (`tests/adapters/`).
- Shared case-table types and helpers go in `tests/common/mod.rs`, included
  per test crate (`mod common;` for root suites, `#[path]` from folder roots).
  The file starts with `#![allow(dead_code, unused_macros, unused_imports)]`
  because each crate uses only part of it.
- Each suite file documents at the top which `src/` file(s) it tests.
- Within a layer the ADR 0004 rules apply unchanged: one custom loop per
  suite over a `yield_*_data()` table, constructor macros for readable data,
  every datum carrying a `test_title`, outcomes recorded in a `TestReport`
  and reported once with the case title and expected vs got.

Applied to: `libs/rust/reflect/tests/` (domain.rs, application.rs,
adapters/{scalar,map,sequence,enum_shape,indirection,unknown}.rs),
`libs/rust/json-path/tests/` (domain.rs, application.rs,
adapters/regex_tokenizer.rs), `libs/rust/shared/tests/value_ops.rs` (single
suite, flat file).

## Consequences

- Finding the tests for a source file is mechanical: same relative path under
  `tests/`, and each suite's header comment names the `src/` file under test.
- New modules copy the same skeleton, so test layout is predictable across
  languages (the TS port mirrors it under each lib's test dir).
