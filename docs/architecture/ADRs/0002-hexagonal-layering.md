# ADR 0002 — Hexagonal layering and dependency direction

- Status: accepted
- Date: 2026-11-11

## Context

The reference ports (go-json, js-json, dyn-rt-rs-old-3) mixed parsing,
regexes, and value handling in single packages. We want the Rust port to make
the architecture compiler-enforced rather than convention-enforced, following
`hexagonal-monorepo-showcase`.

## Decision

Layer per bounded context, enforced at crate level:

```
libs/rust/json-domain          std only; Value model, path AST, ports, JsonError
libs/rust/json-application     depends on json-domain only; use cases:
                               parse path, resolve/get/set/delete/for_each, schema ops
libs/rust/json-infra           adapters: serde_json <-> domain Value, regex tokenizer
libs/rust/metadatamodel-*      same three layers, depending on the json layers
```

Rules (mirrored in the root AGENTS.md):

- `serde`/`serde_json` and `regex` appear ONLY in `*-infra` crates and `apps/`
  composition roots.
- Ports are traits defined in the domain crate; implementations live outside.
- One closed error enum per core; adapters translate native errors into it.
- Core tests are zero-IO: in-memory values only, shared vectors from
  `tests/shared/`.

## Consequences

- The path engine and object ops run against the pure domain `Value`, so they
  are testable without serde and reusable by any future adapter.
- Swapping the regex tokenizer or the serde backend is a local change in one
  infra crate behind a port.
