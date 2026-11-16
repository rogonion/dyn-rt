# AGENTS.md — dyn-rt

Guidance for AI coding agents working in this repository. Read fully before
your first change. Nested `AGENTS.md` files may add narrower rules; the
nearest one wins for the paths it covers.

## Project purpose

`dyn-rt` is the dynamic-runtime library: a dynamic layer over a language's
own data types (built-ins and user structs/classes) — JSONPath-style paths
(`path`), runtime reflection (`reflect`), schemas (`schema`), and
get/set/delete manipulation (`manipulation`) — plus a metadata-model layer on
top (`metadatamodel`). `json` names only the JSON data model (`json::Value`)
and JSON text (de)serialization (ADR 0010). It is a guided port of the
reference implementations in `../go-json`, `../go-metadatamodel`,
`../js-json`, `../js-metadatamodel`. Rust first (`libs/rust`, `packages/rust`),
then a TypeScript mirror (`libs/typescript`, `packages/typescript`). Each
ecosystem ships ONE package (`dyn-rt` crate, `@dyn-rt/*` npm) with
feature/subpath gating so consumers only bundle what they use (ADR 0003).

## Layout (ADR 0001)

```
apps/<lang>/<name>/       executables / composition roots — wire adapters into the core here
libs/<lang>/<module>/     ONE crate per module (ADR 0007): shared, test-support (dev-only),
                          path, manipulation, schema, reflect, metadatamodel.
                          Inside each module crate: src/domain, src/application, src/adapters
                          (hexagon; adapters is the only place regex/serde may be imported).
                          metadatamodel has one submodule per subdomain under src/, each with
                          its own domain/application/adapters dirs.
packages/<lang>/          publishable export entrypoints (façades over libs); for Go: ONE module at
                          packages/go/ with contexts as subpackages (.../packages/go/json), released
                          via subdir-prefixed tags packages/go/vX.Y.Z, local dev via root go.work (ADR 0006)
docs/architecture/        ADRs (decisions) + descriptions/ (long-form)
misc/                     scratch notes, drawio MCP notes
```

## Tooling

- Rust: cargo workspace at the repo root. `cargo test --workspace`,
  `cargo clippy --workspace --all-targets`, `cargo fmt --all`.
- TypeScript: pnpm workspace. `pnpm -r test`, `pnpm -r build`.
- Root orchestration is uv + poe (Python is the task runner only; no Python
  ships in this repo): `uv run poe test` / `lint` / `fmt` / `build`
  (see `poe_tasks.toml`). Python is uv's job: `uv run`, `uv add`, `uv lock`;
  commit `uv.lock`, `.python-version`, `Cargo.lock`, `pnpm-lock.yaml`.
  Never system Python or pip.
- Host needs only: cargo, node + pnpm, uv.

## Testing (ADR 0004)

- Tests are data-driven by default, in EVERY language, with the data written
  in that language as an in-code case table close to the module under test
  (the module crate's own `tests/` dir — e.g. the path parse suite lives
  in `libs/rust/path/tests/`). Tiers (ADR 0004):
  1. In-code case table in the module's own `tests/` dir — the DEFAULT. Each
     suite is one custom loop over a `yield_test_data()`-style table; every
     datum carries the shared `TestMetadata` (at least a `test_title`), and on
     failure the loop reports the case title with expected vs got, collecting
     all failures into one panic summary. Adding a case = adding one entry.
  2. Crate-local `testdata/` — only for cases not representable in source,
     still a case table iterated in one loop.
  3. Inline tests — only when the test is not data at all, and only with an
     explicit reason in a comment above them.
  The harness (`TestMetadata`, `new_test_metadata!`, `new_test_title!`,
  `report_failures`) lives once in `libs/rust/test-support` (dev-only), plus
  per-language equivalents; case-data constructor macros (e.g.
  `json_path_key_selector_segment!`) live in the module crates. Parity
  between languages is by mirroring the same tables, not shared JSON files.
  Test files mirror the crate's layers (ADR 0009): `tests/domain.rs`,
  `tests/application.rs`, `tests/adapters/<file>.rs` per `src/adapters/` file,
  shared helpers in `tests/common/mod.rs`; each suite's header names the
  `src/` file(s) under test.
- `serde_json` may appear in `dev-dependencies` and in
  `libs/rust/test-support` ONLY to parse/emit fixtures — never in shipped
  domain/application code.
- "Zero IO" means no network, subprocess, or external services; reading local
  fixture files is fine.
- When a runtime error is reported, FIRST add a case reproducing the scenario
  to the relevant case table, then fix the code — the suite then pins both
  the old and the new behavior.

## Code style (ADR 0005)

- Prefer explicit, linear logic over brevity or cleverness: named steps, early
  returns, plain loops, one explicit `match` arm per case. Tolerate duplication
  (like the go/js splitter trio) over clever shared abstractions.
- Tokenizing/splitting uses named, single-purpose regexes applied in an
  explicit ordered sequence of steps (dyn-rt-rs-old-3 style) — NOT a
  handwritten char-by-char loop (dyn-rt-rs-old style). No regex golf or dense
  iterator chains. Declarative macros and de-facto standard proc-macro crates
  (serde, thiserror, ...) are fine; bespoke clever macros/proc-macros are not.
  Recursion only when it mirrors the parsed structure, and each step stays
  obvious.
- Optimize only when a profiler or failing test says so, keeping the explicit
  shape. Exception: public API ergonomics (constructors like
  `Value::object(...)`) may be concise; internal logic stays explicit.

## Architecture rules (ADR 0002 + ADR 0007)

- Dependencies point inward. Between modules this is compiler-enforced (crate
  = module, ADR 0007): e.g. `manipulation` may depend on `path` and
  `shared`, never the reverse. Inside a module crate, layers are dirs:
  `domain` (std only) <- `application` <- `adapters`/apps. Never invert.
- `serde` / `serde_json` and `regex` are allowed ONLY in a module's
  `src/adapters/` dir and in `apps/` composition roots. `domain` and
  `application` never import them.
- Ports are roles the core needs; definitions live in the module's domain,
  implementations live in its adapters or in apps.
- Errors: each module core defines one closed error enum. Adapters translate
  native failures into it. No `anyhow` across boundaries.
- Core code does no IO at all; what tests may read is defined by the Testing
  rules above (local fixtures only).
- A shape moves into `shared` only when 2+ modules need it; otherwise keep it
  in its own module.
