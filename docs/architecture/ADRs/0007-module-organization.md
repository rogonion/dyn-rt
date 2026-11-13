# ADR 0007 — Module organization inside the monorepo

- Status: accepted
- Date: 2026-11-11
- Amends: ADR 0001 (layout), ADR 0002 (layering granularity)

## Context

The library has numerous modules, and the json-* ones must be usable
independently (json-path, json-manipulation, json-schema, reflect), while
metadatamodel is one module with independently usable subdomains (core,
unflattener, iter, flattener, filter, fieldcolumns, database). We still want
one export per ecosystem (ADR 0003). How do modules map to crates/dirs?

## Decision

- `libs/rust/<module>/` is ONE crate per module — the independence boundary.
  Inter-module dependencies are therefore compiler-enforced:
  `shared`, `test-support` (dev-only), `json-path`, `json-manipulation`,
  `json-schema`, `reflect`, `metadatamodel`.
- Inside each module crate, the hexagon is a directory convention:
  `src/domain/` (models, ports, error vocab — std only), `src/application/`
  (use cases; imports domain only), `src/adapters/` (the ONLY place `regex`
  / `serde` may be imported in the crate). Intra-crate layering is enforced
  by rule + review (the compiler cannot see inside a crate); inter-module
  layering remains compiler-enforced at crate level.
- `shared` (crate `dyn-rt-shared`) holds the substrate all json modules use:
  `Value`, `Number`, the json error vocabulary, and future shared helpers.
  `test-support` stays dev-only and hosts the shared test harness
  (`TestReport`, `new_test_title!`, `report_failures` — ADR 0004).
- `metadatamodel` is ONE crate (`dyn-rt-metadatamodel`) with one submodule per
  subdomain under `src/` (`core`, `unflattener`, `iter`, `flattener`,
  `filter`, `fieldcolumns`, `database`); each submodule is internally
  organized as `domain/` `application/` `adapters/` dirs and is usable
  independently through the façade.
- The façade `packages/rust/dyn-rt` keeps ADR 0003: one crate, per-module
  cargo features (`json-path`, `json-manipulation`, `json-schema`, `reflect`,
  `metadatamodel`) plus group features (`json`, `metadatamodel`, default
  `json`), exposing `dyn_rt::json::{path, manipulation, schema}`,
  `dyn_rt::reflect`, `dyn_rt::metadatamodel::{...}`.
- TypeScript mirrors this: `libs/typescript/<module>/` with the same internal
  dirs; `packages/typescript/` is one npm package with subpath exports
  (`./json/path`, `./json/manipulation`, ..., `./metadatamodel/...`).

## Consequences

- json-* modules compile and publish independently; metadatamodel subdomains
  are independently importable from one crate.
- One repo, one export per ecosystem; adding a module = new crate dir +
  one feature/subpath in the façade.
- Cost accepted: intra-crate hexagon layers are convention, not compiler-
  enforced (unlike the earlier per-layer crates); mitigated by AGENTS.md
  rules and review.
