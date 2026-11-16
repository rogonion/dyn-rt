// Shared case-table types for the path suites — the TS equivalent of the
// Rust `*_case!` constructor macros: each case is a plain object with an
// optional `testTitle`, and each suite is one `test(...)` looping a
// `yield*Data()` table (ADR 0004/0009).

import type { PathSegment, RecursiveDescentSegments } from "../src/index.js";

/** Case for the three split steps: source path -> expected parts. */
export interface SplitCase {
  testTitle?: string;
  source: string;
  expected: string[];
}

/** Case for extractSegment: source token -> expected PathSegment. */
export interface ExtractCase {
  testTitle?: string;
  source: string;
  expected: PathSegment;
}

/** Case for parsePath: source path -> expected AST. */
export interface ParseCase {
  testTitle?: string;
  source: string;
  expected: RecursiveDescentSegments;
}

/** Case for rendering: AST -> expected path string (round-trip). */
export interface RenderCase {
  testTitle?: string;
  segments: RecursiveDescentSegments;
  roundtrip: string;
}

/** Structural equality for ASTs (plain data, so JSON comparison works). */
export function astEquals(a: unknown, b: unknown): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}
