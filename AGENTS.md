# AGENTS.md — dyn-rt

Guidance for AI coding agents working in this repository. Read fully before
your first change. Nested `AGENTS.md` files may add narrower rules; the
nearest one wins for the paths it covers.

## Project purpose

`dyn-rt` is the dynamic-runtime library: a JSONPath-capable JSON toolkit
(`json`) plus a metadata-model layer on top (`metadatamodel`). It is a guided
port of the reference implementations in `../go-json`, `../go-metadatamodel`,
`../js-json`, `../js-metadatamodel`. Rust first (`libs/rust`, `packages/rust`),
then a TypeScript mirror (`libs/typescript`, `packages/typescript`). Each
ecosystem ships ONE package (`dyn-rt` crate, `@dyn-rt/*` npm) with
feature/subpath gating so consumers only bundle what they use (ADR 0003).

## Layout (ADR 0001)

```
apps/<lang>/<name>/       executables / composition roots — wire adapters into the core here
libs/<lang>/<name>/       internal code, layered per ADR 0002 (Rust: one crate per layer)
packages/<lang>/          publishable export entrypoints (façades over libs); for Go: ONE module at
                          packages/go/ with contexts as subpackages (.../packages/go/json), released
                          via subdir-prefixed tags packages/go/vX.Y.Z, local dev via root go.work (ADR 0006)
tests/shared/             cross-language fixtures consumed by cargo tests AND vitest (parity)
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

- Tests are data-driven by default, in one of three tiers (ADR 0004):
  1. `tests/shared/**/*.json` — cross-language vectors; MUST be
     JSON-representable (both languages load the same files). Non-JSON
     constructs go through a `build` instruction (e.g. `{"object": [[k, v]]}`
     for duplicate keys).
  2. Crate-local `testdata/` — language-specific, non-JSON cases, still a
     case table iterated in one loop.
  3. Inline tests — only when the test is not data at all, and only with an
     explicit reason in a comment above them.
  Every case has a `name` and is reported by name on failure, with expected
  vs got printed — the go/js `it.each` / range-over-func shape.
- `serde_json` may appear in `dev-dependencies` and in
  `libs/rust/test-support` ONLY to parse/emit fixtures — never in shipped
  domain/application code.
- "Zero IO" means no network, subprocess, or external services; reading local
  fixture files is fine.
- When a runtime error is reported, FIRST add a case reproducing the scenario
  to the relevant `tests/shared` vector file, then fix the code — the suite
  then pins both the old and the new behavior.

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

## Architecture rules (ADR 0002)

- Dependencies point inward: `*-domain` crates depend on nothing but std;
  `*-application` crates import only `*-domain` crates; adapters and apps
  import both. This is enforced at compile time by the workspace dependency
  graph — never invert it.
- `serde` / `serde_json` and `regex` are allowed ONLY in `*-infra` crates and
  in `apps/` composition roots. Domain and application crates never see them.
- Ports are roles the core needs; definitions live in the domain crate,
  implementations live in infra/app crates.
- Errors: each core defines one closed error enum (`JsonError`, ...).
  Adapters translate native failures into it. No `anyhow` across boundaries.
- Core code does no IO at all; what tests may read is defined by the Testing
  rules above (local fixtures only). Shared vectors live in `tests/shared/`.
- A shape moves into a shared core only when 2+ contexts need it; otherwise
  keep it in its own context.
