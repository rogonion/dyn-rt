// `DynObject`: the stateful entrypoint wrapper over the pure get / set /
// deleteAt / forEach use cases (port of
// libs/rust/manipulation/src/application/object.rs — the analogue of go's
// object.Object / js's JSObject, named DynObject because `Object` clashes with
// TypeScript's global Object). It wraps a source value plus an optional schema
// and accepts a JSONPath in any representation (text, parsed segment(s)), so
// callers do not have to parse paths themselves. It deliberately holds no
// result state; a future access-index module can be held here as an optional
// field for direct-access execution.
//
// TS note: "borrowed" vs "owned" is conceptual — a TS reference is shared
// either way — but the wrapper still tracks read-only-ness (set/delete on a
// read-only source throw) and can hold the source itself (owned) so there is
// no separate source owner to deal with; intoSource() hands it back. Root-only
// set/delete reassign the held source, which is what makes replacing a
// primitive root possible (the pure functions can only reset containers in
// place).

import {
  parsePath,
  RegexPathTokenizer,
  type RecursiveDescentSegment,
  type RecursiveDescentSegments,
} from "@dyn-rt/path";
import { reflectZero } from "@dyn-rt/reflect";
import type { Schema, SchemaNode } from "@dyn-rt/schema";
import { ManipulationError } from "../error.js";
import { areEqual } from "./are_equal.js";
import { deleteAt } from "./delete.js";
import { forEach } from "./for_each.js";
import { get } from "./get.js";
import { convertAtPath, isRootOnly } from "./helpers.js";
import { set } from "./set.js";

/** A JSONPath in any accepted representation: text (parsed on the spot via
 * path's tokenizer), a single parsed recursive-descent segment, or parsed
 * segments. */
export type PathInput = string | RecursiveDescentSegment | RecursiveDescentSegments;

/** Resolves a PathInput to parsed segments (text is parsed here). An empty
 * array is treated as empty segments (i.e. the root). */
export function resolvePathInput(input: PathInput): RecursiveDescentSegments {
  if (typeof input === "string") {
    return parsePath(new RegexPathTokenizer(), input);
  }
  if (input.length === 0) return [];
  if (Array.isArray(input[0])) return input as RecursiveDescentSegments;
  return [input as RecursiveDescentSegment];
}

/** The manipulation entrypoint: a source value (shared read-only or held) plus
 * an optional schema, with get / set / delete / forEach methods taking a
 * JSONPath in any PathInput representation. Thin wrapper over the pure
 * application use cases. */
export class DynObject {
  private source: unknown;
  private readonly readOnlyFlag: boolean;
  private schema: Schema | undefined;

  private constructor(source: unknown, readOnly: boolean) {
    this.source = source;
    this.readOnlyFlag = readOnly;
    this.schema = undefined;
  }

  /** Wrap a borrowed mutable source (the common case: pass your own value). */
  static new(source: unknown): DynObject {
    return new DynObject(source, false);
  }

  /** Wrap a shared (read-only) source; set/delete on it throw. */
  static readOnly(source: unknown): DynObject {
    return new DynObject(source, true);
  }

  /** Wrap an owned source (no external owner needed); hand it back with
   * intoSource(). */
  static owned(source: unknown): DynObject {
    return new DynObject(source, false);
  }

  /** Attach a schema (guides set coercion/creation). */
  withSchema(schema: Schema): this {
    this.schema = schema;
    return this;
  }

  /** Attach an optional schema. */
  withSchemaOpt(schema: Schema | undefined): this {
    this.schema = schema;
    return this;
  }

  /** Attach a schema node. */
  withSchemaNode(node: SchemaNode): this {
    this.schema = node;
    return this;
  }

  /** The wrapped source. */
  getSource(): unknown {
    return this.source;
  }

  /** Consume the wrapper, returning the (possibly modified) source. */
  intoSource(): unknown {
    return this.source;
  }

  /** Get the value(s) at `path` (see get). */
  get(path: PathInput): unknown[] {
    return get(this.source, resolvePathInput(path));
  }

  /** Set `value` at `path` using the attached schema; returns how many values
   * were set. */
  set(path: PathInput, value: unknown): number {
    return this.setWithSchema(path, value, this.schema);
  }

  /** Set `value` at `path` with a per-call schema (overrides the attached one
   * for this call); returns how many values were set. */
  setWithSchema(path: PathInput, value: unknown, schema: Schema | undefined): number {
    this.assertWritable("set");
    const segments = resolvePathInput(path);
    if (isRootOnly(segments)) {
      // a root set replaces the held source (works for primitives too)
      this.source = convertAtPath(value, [], schema);
      return 1;
    }
    return set(this.source, segments, value, schema);
  }

  /** Delete the value(s) at `path`; returns how many were deleted. */
  delete(path: PathInput): number {
    this.assertWritable("delete");
    const segments = resolvePathInput(path);
    if (isRootOnly(segments)) {
      this.source = reflectZero(this.source);
      return 1;
    }
    return deleteAt(this.source, segments);
  }

  /** Call `callback` for each value found at `path`; a `false` return
   * terminates the walk. */
  forEach(path: PathInput, callback: (value: unknown) => boolean): void {
    forEach(this.source, resolvePathInput(path), callback);
  }

  /** Deep structural equality helper exposed for convenience (areEqual). */
  static areEqual(left: unknown, right: unknown): boolean {
    return areEqual(left, right);
  }

  private assertWritable(op: string): void {
    if (this.readOnlyFlag) {
      throw ManipulationError.valueAtPathSegmentInvalid(
        `cannot ${op} on a read-only source (DynObject.readOnly)`,
      );
    }
  }
}
