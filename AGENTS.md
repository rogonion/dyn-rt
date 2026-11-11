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
packages/<lang>/          publishable export entrypoints (façades over libs)
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

- Tests are data-driven by default: a suite is a table of cases loaded from
  `tests/shared/**/*.json` (cross-language vectors) or a crate-local
  `testdata/` dir, iterated in a single loop. Every case has a `name` and is
  reported by name on failure, with expected vs got printed — the go/js
  `it.each` / range-over-func shape, sourced from shared JSON so Rust and
  TypeScript run identical cases.
- Inline case-by-case tests need an explicit reason in a comment above them.
- `serde_json` may appear in `dev-dependencies` and in
  `libs/rust/test-support` ONLY to parse/emit fixtures — never in shipped
  domain/application code.
- "Zero IO" means no network, subprocess, or external services; reading local
  fixture files is fine.

## Code style (ADR 0005)

- Prefer explicit, linear logic over brevity or cleverness: named steps, early
  returns, plain loops, one explicit `match` arm per case. Tolerate duplication
  (like the go/js splitter trio) over clever shared abstractions.
- No regex golf, dense iterator chains, or macros/generics used for brevity.
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
