# dyn-rt

The dynamic-runtime library: a JSONPath-capable JSON toolkit (`json`) plus a
metadata-model layer on top (`metadatamodel`). Guided port of the reference
implementations in `../go-json`, `../go-metadatamodel`, `../js-json`,
`../js-metadatamodel` — Rust first, then TypeScript.

Each ecosystem ships one package with per-context gating (tree-shaking):
the `dyn-rt` crate (cargo features) and the `@dyn-rt/*` npm package (subpath
exports). See `docs/architecture/ADRs/` for the decisions.

## Layout

```
apps/       executables (composition roots)
libs/       one crate per module (ADR 0007): shared, test-support (dev-only),
            path, manipulation, schema, reflect, metadatamodel —
            each with src/{domain, application, adapters} inside
packages/   publishable export entrypoints (one per ecosystem, feature/subpath gated)
docs/       architecture ADRs + descriptions
```

## Commands

Run everything from the repo root via uv + poe:

```sh
uv run poe fmt     # cargo fmt
uv run poe lint    # cargo clippy + pnpm lint
uv run poe test    # cargo test --workspace + pnpm -r test
uv run poe build   # cargo build --workspace + pnpm -r build
```

Or directly: `cargo test --workspace` for Rust, `pnpm -r test` for TypeScript.
