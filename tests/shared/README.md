# Shared cross-language test fixtures (ADR 0004)

Every suite is data-driven: a table of cases in a JSON file under this
directory, loaded and iterated by the Rust (`cargo test`) and TypeScript
(vitest) suites alike, so both ports are held to identical behavior. Each case
has a `name`; failures are reported per case with expected vs got printed.

Format: each file is a JSON array of case objects. Suites define their own keys,
but the common shape is:

```json
[
  {
    "name": "object member lookup dedupes keys",
    "build": { "object": [["a", 1], ["b", "two"], ["a", 3]] },
    "assert": { "type": "object", "get": { "a": 3, "b": "two" }, "get_missing": ["zz"] }
  }
]
```

- `build` / `input` describe how to construct the subject of the case
  (`{"value": <any json>}` wraps a literal; `{"object": [[key, value], ...]}`
  builds an object from ordered pairs, allowing duplicate keys).
- `assert` / `expected` describe what to check. The harness in each language
  maps these keys onto the API under test.

Layout: `tests/shared/<context>/<suite>.json`, e.g. `json/value-ops.json`,
`json/path-parse.json`.

Rules:

- Add a case here first when pinning or fixing a behavior, then make both
  language suites pass it.
- Cases that JSON cannot express and that are language-specific live in the
  crate's own `testdata/` directory.
