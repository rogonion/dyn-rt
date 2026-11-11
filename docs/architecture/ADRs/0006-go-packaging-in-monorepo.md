# ADR 0006 — Go packaging inside the monorepo (git-based distribution)

- Status: accepted
- Date: 2026-11-11

## Context

Rust and TypeScript have package registries (crates.io, npm), so
`packages/<lang>/` can be a normal publishable unit. Go distributes modules
through git itself: a module is "repo URL + subdirectory", and a nested
directory cannot be its own git repo without submodule pain. We still want
the ADR 0003 shape — one package per ecosystem with per-context gating — and
we must decide how intra-monorepo dependencies are expressed without
"git-source" hacks.

## Decision

- `packages/go/` is ONE Go module: `go.mod` at `packages/go/` with module
  path `github.com/rogonion/dyn-rt/packages/go`. The contexts are
  subpackages: `packages/go/json`, `packages/go/metadatamodel`, giving import
  paths `github.com/rogonion/dyn-rt/packages/go/json` etc. This mirrors
  `packages/rust/dyn-rt` (one crate, feature-gated modules) and
  `packages/typescript` (one npm package, subpath exports). Go's linker
  dead-code-eliminates unimported packages, so separate modules per context
  are unnecessary — that is Go's tree-shaking.
- Releases use subdir-prefixed tags: `packages/go/vX.Y.Z` (Go requires this
  form for modules rooted in a repo subdirectory). One tag releases the whole
  go module.
- Intra-monorepo development uses a root `go.work` (`use ./packages/go`,
  `./apps/go/...`), committed for repo development; `go work sync` aligns
  versions. Never commit `replace` directives with relative paths into
  published modules, and never depend on a sibling module by commit hash or
  pseudo-version — `go.work` covers local resolution; consumers only ever see
  tagged requires.
- External consumers use plain `go get github.com/rogonion/dyn-rt/packages/go/json`.
  For public modules this resolves via the module proxy (which itself fetches
  from git) — no git-source dependency needed. For PRIVATE repos, set
  `GOPRIVATE=github.com/rogonion/*` (or `GOVCS`) so resolution goes direct to
  git with auth; that is the only case where "dependency by git source"
  applies, and it is an environment setting, not a per-dependency hack.
- Optional later: a vanity import path (e.g. `dyn-rt.dev/go`) via `go-import`
  meta tags. Not required; skip unless we want prettier import paths.
- Nested git repos / submodules for `packages/go` are rejected.

## Consequences

- One git repo serves all three ecosystems: crates.io for Rust, npm for
  TypeScript, module proxy (or direct git when private) for Go.
- Go consumers get one module with subpackage imports; versioning is
  per-module via `packages/go/vX.Y.Z` tags.
- No consumer ever pins an untagged commit of a monorepo module; local dev is
  `go.work`, never `replace`.
