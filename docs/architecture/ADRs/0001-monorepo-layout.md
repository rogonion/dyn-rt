# ADR 0001 — Monorepo layout

- Status: accepted
- Date: 2026-11-11

## Context

dyn-rt must serve two ecosystems (Rust, TypeScript) from one repo so the ports
stay in lockstep, following the conventions already used in this workspace of
projects (`hexagonal-monorepo-showcase`, `postgres-setup`, `icipe-portfolio`).

## Decision

One repo, dual toolchains, four top-level code directories:

- `apps/<lang>/<name>/` — executables and composition roots (wire adapters
  into the core).
- `libs/<lang>/<name>/` — internal code. Rust: one crate per hexagonal layer
  (`json-domain`, `json-application`, `json-infra`, ...). TypeScript: one
  workspace lib per context with `domain/ application/ adapters/` inside.
- `packages/<lang>/` — publishable export entrypoints (façades re-exporting
  the libs).
- Test data lives in-language, inside each module's own `tests/` dir (ADR 0004);
  there is no shared vector directory — languages mirror the same case tables.

Root tooling: cargo workspace (`apps/rust/*`, `libs/rust/*`, `packages/rust/*`)
+ pnpm workspace (`libs/typescript/*`, `packages/typescript/*`), orchestrated
by uv + poe (`poe_tasks.toml`, dev dependency group in `pyproject.toml`).
Python is the task runner only. `docs/architecture/` holds ADRs.

## Consequences

- One `Cargo.lock` / one `pnpm-lock.yaml` / one `uv.lock` at the root.
- Cross-language behavior drift is caught by shared fixtures, not by review.
- poe tasks are the single entry point (`uv run poe test|lint|fmt|build`).
