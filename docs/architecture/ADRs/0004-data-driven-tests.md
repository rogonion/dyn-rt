# ADR 0004 — Data-driven tests

- Status: accepted
- Date: 2026-11-11

## Context

The reference implementations drive their suites from case tables written in
the implementation language: go-json ranges over test-data iterators yielding
`{input, expected}` structs, js-json uses `it.each(testData)` arrays.
dyn-rt-rs-old-3 showed the Rust shape we adopt: per-suite custom loops over
`yield_test_data()` tables whose entries are built by constructor macros and
carry a `test_title` field. An earlier revision of this ADR routed
cases through `tests/shared/**/*.json` for cross-language parity, but JSON
fixtures make tests unreadable as usage examples and force every case through
a JSON encoding; in-language tables show accepted inputs directly.

## Decision

- In EVERY language (Rust, TypeScript, Go, ...), tests are data-driven by
  default and the test data is written in that language, in code, close to the
  module under test (the module crate/package's own `tests/` dir — e.g. the
  json-path parse suite lives in `libs/rust/json-path/tests/`). Where the
  tests sit inside `tests/` mirrors the crate's layers — see ADR 0009.
- Every suite is a custom loop over a case table — no framework-enforced
  shape. Typically one `#[test]` per suite iterating a `yield_test_data()`-style
  table; adding a new case means adding one entry to the table in code.
- Every test datum is a plain case struct carrying its own `test_title`
  (`Option<String>`) field. The suite loop registers each case in a
  `TestReport` (`begin_case(title)`) and marks mismatches with
  `fail_case(detail)`. The report has a root suite title + description and
  logs one line per case, pass or fail:
  `Test case -> N/M : [[[ title ]]] || PASS` (failures add `|| FAIL ||
expected ... ; got ...`). All cases are evaluated, then `finish()` panics
  once with the failure summary if any case failed; opting into
  `abort_on_failure()` panics on the first failing case instead.
- Constructor macros for building case data are the sanctioned way to keep
  tables readable (ADR 0005 allows declarative macros): e.g. the
  `json_path_root_segment!` / `json_path_key_selector_segment!` /
  `new_recursive_descent_segment(s)!` macros in `dyn-rt-json-path` and the
  per-suite case macros (`split_case!`, `value_case!`, ...). The harness
  pieces (`TestReport`, `new_test_title!`, plus the legacy `report_failures`
  delegate) live once in `libs/rust/test-support` (dev-only); each other
  language gets its own equivalent.
- Tiers:
  1. In-code case table in the module's own `tests/` dir — the DEFAULT.
  2. Crate-local `testdata/` files — only for cases not representable in
     source, still loaded as a case table and iterated in one loop.
  3. Inline tests — only when the test is not data at all, and only with an
     explicit reason in a comment directly above the test.
- Cross-language parity is achieved by mirroring the same case tables in each
  language (same cases, same titles), not by sharing JSON files.
  `tests/shared/` has been removed.
- When a runtime error is reported, FIRST add a case reproducing the scenario
  to the relevant table, then fix the code — the suite then pins both the old
  and the new behavior.
- `serde_json` may appear in `dev-dependencies` and in
  `libs/rust/test-support` solely to parse/emit fixtures — never in shipped
  domain or application code.
- Tests may read local fixture files. "Zero IO" means no network, subprocess,
  or external services.

## Consequences

- Tests double as usage examples: a reader sees accepted inputs and expected
  outputs directly in the language of the library.
- Suites stay thin loops over tables; the harness lives once in
  `dyn-rt-test-support` (plus per-language equivalents).
- Rust and TypeScript mirror the same tables case-for-case; parity is a
  review-time check of the tables, not a shared file format.
