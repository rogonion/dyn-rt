# Shared cross-language test fixtures

JSON vector files consumed by BOTH the Rust suites (`cargo test`) and the
TypeScript suites (vitest), so the two ports are held to identical behavior.

Each file is a JSON array of cases, e.g.:

```json
[
  {
    "name": "root object member",
    "input": { "a": { "b": [1, 2, 3] } },
    "path": "$.a.b[1]",
    "expected": 2
  }
]
```

Conventions:

- Suites live next to their code (`libs/rust/*/tests/`, `libs/typescript/*/tests/`)
  and read these files through a relative path (`../../../tests/shared/...`).
- `path` cases: `expected` is the resolved value; parser suites may instead
  compare a normalized segment structure.
- Add a case here first when fixing a bug or pinning behavior — then make both
  language suites pass it.
