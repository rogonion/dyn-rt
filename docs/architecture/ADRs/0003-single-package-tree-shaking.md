# ADR 0003 — One package per ecosystem, contexts gated for tree-shaking

- Status: accepted
- Date: 2026-11-11

## Context

The old libraries shipped as separate packages per library
(`@rogonion/js-json`, `@rogonion/js-metadatamodel`, `go-json`,
`go-metadatamodel`). The goal for dyn-rt is a single dynamic-runtime package
that still lets consumers pull only the `json` or `metadatamodel` context —
Rust and TypeScript (and Go) both support this via dead-code elimination /
tree-shaking.

## Decision

- Rust: one publishable crate `dyn-rt` in `packages/rust/dyn-rt/` that
  re-exports the lib crates as modules (`dyn_rt::json`, `dyn_rt::metadatamodel`),
  with cargo features `json` (default) and `metadatamodel` selecting what
  compiles. Internal layer crates (`dyn-rt-json-domain`, ...) stay in `libs/`.
- TypeScript: one npm package under `packages/typescript/` exposing subpath
  exports (`@dyn-rt/.../json`, `@dyn-rt/.../metadatamodel`) as pure ESM so
  bundlers tree-shake unused contexts.
- `packages/` is the only publishable surface; `libs/` crates are internal
  (not published on their own).

## Consequences

- Consumers depend on one name per ecosystem.
- Adding a context = a new feature/subpath + module in the façade, not a new
  package.
- The façade must stay thin (re-exports only) so layering rules in ADR 0002
  are not bypassed.
