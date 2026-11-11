# ADR 0005 — Explicit logic over cleverness

- Status: accepted
- Date: 2026-11-11

## Context

The reference implementations (go-json, js-json) contain regex-driven
splitters with manual match-index bookkeeping, and recursive helpers whose
control flow is debatable — compact, but hard to follow. Optimizing these into
cleverer abstractions would cost human readability and make the port harder to
verify against the references line by line.

## Decision

When porting or writing logic in dyn-rt, prefer explicitness and simple,
linear logic flow over brevity:

- Write the straightforward version: named steps, early returns, plain loops.
  Tolerate duplication (e.g. similar split passes for `..`, `.`, and
  brackets) over a shared clever abstraction — the go/js splitter trio is the
  model, not a problem to solve.
- Tokenizing/splitting prefers NAMED, single-purpose regexes applied in an
  explicit, ordered sequence of steps — the dyn-rt-rs-old-3 style, where each
  regex constant has a name and each split pass is a separate visible call.
  This is preferred over a handwritten character-by-character state machine
  (the dyn-rt-rs-old style), whose implicit state is harder to follow.
- No regex golf (one mega-pattern with many capture groups), no dense iterator
  chains, no bespoke macros or generics written for cleverness. A `match` with
  one explicit arm per case beats a clever fold.
- Rust's deduplication mechanics are welcome where they are idiomatic and
  explainable: declarative macros for repetitive construction (the
  `json_path_*_segment!` constructor macros in dyn-rt-rs-old-3 are the model)
  and de-facto standard proc-macro crates (serde, thiserror, ...) are fine.
  Bespoke proc-macros written for this repo are not. Explicitness governs
  logic flow — it does not mean rejecting Rust idioms.
- Recursion is fine when it mirrors the structure being parsed (a path is a
  list of segments), but keep each recursive step obvious; prefer a `for` loop
  over the recursion when both are clear.
- Optimize only when a profiler or a failing test says so, and then keep the
  explicit structure — change constants and data structures, not the shape of
  the logic.
- The exception to "no cleverness" is the public API ergonomics (e.g.
  `Value::object(...)` constructors), which may be concise because they are
  used everywhere; internal logic stays explicit.

## Consequences

- Code reads top-to-bottom; each function does one obvious thing.
- Ports can be reviewed against go-json/js-json almost line by line.
- Some duplication is expected and accepted — do not "clean it up" into
  abstractions during a port.
