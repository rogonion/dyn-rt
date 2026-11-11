# ADR 0004 — Data-driven tests

- Status: accepted
- Date: 2026-11-11

## Context

The reference implementations drive their suites from case tables: go-json
ranges over test-data iterators yielding `{input, expected}` structs, js-json
uses `it.each(testData)` arrays, and go-metadatamodel keeps document fixtures in
`testdata/*.json`. dyn-rt-rs-old-3 instead scattered inline `#[test]`s. We want
one convention across the repo — and, since Rust and TypeScript ports must
behave identically, one shared source of cases.

## Decision

- Every suite is data-driven by default: a table of cases loaded from
  `tests/shared/**/*.json` (canonical cross-language vectors) or a crate-local
  `testdata/` directory, iterated in a single loop. Each case carries a `name`;
  on failure the test reports the case name and prints expected vs got.
- Inline, case-by-case tests are allowed only with an explicit reason in a
  comment directly above the test.
- `serde_json` may appear in `dev-dependencies` and in
  `libs/rust/test-support` solely to parse/emit fixtures — never in shipped
  domain or application code.
- Tests may read local fixture files. "Zero IO" means no network, subprocess,
  or external services.

## Consequences

- Rust and TypeScript run identical vectors; pinning or fixing a behavior means
  adding one JSON row and making both suites pass it.
- Fixture loading and serde_json <-> domain `Value` conversion for tests live
  once in `dyn-rt-test-support`; suites stay thin loops over cases.
- Language-specific cases that JSON cannot express (e.g. duplicate object keys
  pre-parse) go in the crate's own `testdata/` and stay in that language — or
  are expressed through a `build` instruction the harness understands.
