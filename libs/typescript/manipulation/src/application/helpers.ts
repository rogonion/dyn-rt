// Shared traversal helpers for the get / set / delete use cases (port of
// libs/rust/manipulation/src/application/mod.rs). TS values are plain
// references routed by kindOf: maps are plain objects (string keys), structs
// are class instances, sequences are arrays. The only "indirection" shape is
// the enum value { variant, payload? } produced by the schema module, whose
// "Some" payload unwraps on traversal (the analogue of Box / Option::Some).

import type {
  PathSegment,
  PathSelector,
  RecursiveDescentSegments,
  SliceSelector,
} from "@dyn-rt/path";
import { asMap, asSequence, asStruct, mapKeys, structFieldNames } from "@dyn-rt/reflect";
import { isPlainObject, kindOf } from "@dyn-rt/shared";
import { convert, isSchemaSet, schemaAtPath, type Schema, type SchemaNode } from "@dyn-rt/schema";
import { ManipulationError } from "../error.js";

// ---------------------------------------------------------------------------
// indirection unwrapping
// ---------------------------------------------------------------------------

/** True for an enum-shaped value ({ variant: string, payload? }) — the shape
 * the schema module's enum conversion produces. */
export function isEnumValue(value: unknown): value is { variant: string; payload?: unknown } {
  return (
    isPlainObject(value) &&
    typeof (value as Record<string, unknown>).variant === "string"
  );
}

/** Unwraps one level of indirection: the payload of a `Some` enum value.
 * Returns the value itself for non-wrappers and for other variants. */
export function unwrap(value: unknown): unknown {
  if (isEnumValue(value) && value.variant === "Some") {
    return value.payload;
  }
  return value;
}

/** Unwraps repeatedly (Some(Some(x)) style nesting). */
export function unwrapAll(value: unknown): unknown {
  let current = value;
  for (;;) {
    const next = unwrap(current);
    if (next === current) return current;
    current = next;
  }
}

// ---------------------------------------------------------------------------
// path helpers
// ---------------------------------------------------------------------------

/** Resolves a (possibly negative) path index against a sequence length.
 * Negative indices count from the end (`-1` = last). Throws on out of range. */
export function resolveIndex(index: number, len: number): number {
  if (index >= 0) {
    if (index >= len) {
      throw ManipulationError.valueAtPathSegmentInvalid(
        `index ${index} out of range (length ${len})`,
      );
    }
    return index;
  }
  const idx = len + index;
  if (idx < 0) {
    throw ManipulationError.valueAtPathSegmentInvalid(
      `index ${index} out of range (length ${len})`,
    );
  }
  return idx;
}

/** Like resolveIndex, but a non-negative index at or past the end is allowed
 * (set expands the sequence); only negative-out-of-range throws. */
export function resolveSetIndex(index: number, len: number): number {
  if (index >= 0) return index;
  return resolveIndex(index, len);
}

/** Expands a slice selector `[start:end:step]` into element indices, mirroring
 * go's bounds behavior (an explicit out-of-range bound is an error; a
 * non-positive step behaves as step 1). */
export function sliceIndices(slice: SliceSelector, len: number): number[] {
  const resolveBound = (bound: number, isStart: boolean): number => {
    if (bound >= 0) {
      if (bound >= len) {
        throw ManipulationError.pathSegmentInvalid(
          `slice bound ${bound} out of range (length ${len})${isStart ? " (start)" : " (end)"}`,
        );
      }
      return bound;
    }
    const v = len + bound;
    if (v < 0) {
      throw ManipulationError.pathSegmentInvalid(
        `slice bound ${bound} out of range (length ${len})`,
      );
    }
    return v;
  };

  const start = slice.start !== undefined ? resolveBound(slice.start, true) : 0;
  let step = 1;
  if (slice.step !== undefined && slice.step > 0) {
    if (slice.step >= len) {
      throw ManipulationError.pathSegmentInvalid(
        `slice step ${slice.step} out of range (length ${len})`,
      );
    }
    step = slice.step;
  }
  const end = slice.end !== undefined ? resolveBound(slice.end, false) : len;

  const indices: number[] = [];
  let i = start;
  while (i < end) {
    if (i < len) indices.push(i);
    i += step;
  }
  return indices;
}

/** Resolves the sequence-targeting members of a union selector into concrete
 * indices: index members (negatives resolved against `len`, out-of-range
 * skipped), slice members expanded via sliceIndices, wildcard members expand
 * to all indices. Key members are not sequence-targeting and are ignored here
 * (the map and struct union arms handle those). */
export function unionMemberIndices(members: PathSegment[], len: number): number[] {
  const indices: number[] = [];
  for (const member of members) {
    if (member.selector.kind === "Index") {
      try {
        indices.push(resolveIndex(member.selector.index, len));
      } catch {
        // out-of-range index member: skipped
      }
    } else if (member.selector.kind === "Slice") {
      indices.push(...sliceIndices(member.selector.slice, len));
    } else if (member.selector.kind === "Wildcard") {
      for (let i = 0; i < len; i++) indices.push(i);
    }
  }
  return indices;
}

/** A struct field addressed by a selector: keys by name; non-negative indices
 * by positional field name ("0", "1", ...) — the tuple analogue, where
 * `$[1]` addresses the second positional field. */
export function structFieldBySelector(
  structure: object,
  selector: PathSelector,
): { present: boolean; value?: unknown } {
  const record = structure as Record<string, unknown>;
  if (selector.kind === "Key") {
    return Object.prototype.hasOwnProperty.call(record, selector.key)
      ? { present: true, value: record[selector.key] }
      : { present: false };
  }
  if (selector.kind === "Index" && selector.index >= 0) {
    const name = String(selector.index);
    return Object.prototype.hasOwnProperty.call(record, name)
      ? { present: true, value: record[name] }
      : { present: false };
  }
  return { present: false };
}

/** The child values of a container (map values, sequence elements, struct
 * fields) — used by the wildcard selector. */
export function children(value: unknown): unknown[] {
  const map = asMap(value);
  if (map !== undefined) return mapKeys(map).map((key) => map[key]);
  const seq = asSequence(value);
  if (seq !== undefined) return [...seq];
  const struct = asStruct(value);
  if (struct !== undefined) return structFieldNames(struct).map((name) => struct[name]);
  throw ManipulationError.pathSegmentInvalid(
    `wildcard selector unsupported for kind ${kindLabel(value)}`,
  );
}

/** A human label for the value's kind, for error messages. */
export function kindLabel(value: unknown): string {
  if (isEnumValue(value)) return "enum";
  switch (kindOf(value)) {
    case "array":
      return "sequence";
    case "plain-object":
      return "map";
    case "class-instance":
      return "struct";
    case "date":
      return "date";
    case "null":
    case "undefined":
    case "boolean":
    case "number":
    case "bigint":
    case "string":
      return "scalar";
    default:
      return "unknown";
  }
}

/** The selector's label for error messages. */
export function selectorLabel(selector: PathSelector): string {
  switch (selector.kind) {
    case "Unrecognized":
      return "<unrecognized>";
    case "Root":
      return "$";
    case "Wildcard":
      return "*";
    case "Key":
      return `key ${JSON.stringify(selector.key)}`;
    case "Index":
      return `index [${selector.index}]`;
    case "Slice":
      return `slice ${JSON.stringify(selector.slice)}`;
    case "Union":
      return `union [${selector.members.map((m) => selectorLabel(m.selector)).join(", ")}]`;
  }
}

/** Whether the segments hold anything at all (a lone `$` still has a root
 * segment; a completely empty parse is treated as the root). */
export function isRootOnly(segments: RecursiveDescentSegments): boolean {
  if (segments.length === 0) return true;
  if (segments.length === 1) {
    const group = segments[0]!;
    if (group.length === 0) return true;
    if (group.length === 1 && group[0]!.selector.kind === "Root") return true;
  }
  return false;
}

// ---------------------------------------------------------------------------
// schema-driven defaults and conversion (used by set)
// ---------------------------------------------------------------------------

/** A zero scalar for a scalar type name. */
export function scalarZero(typeName: string): unknown {
  switch (typeName) {
    case "number":
    case "float":
    case "f64":
    case "int":
    case "i64":
      return 0;
    case "bigint":
      return 0n;
    case "boolean":
    case "bool":
      return false;
    case "string":
      return "";
    default:
      return null;
  }
}

/** Reduces a schema to a single node (sets yield their default/first node). */
function schemaAsNode(schema: Schema): SchemaNode {
  if (!isSchemaSet(schema)) return schema;
  const hit = schema.defaultKey !== undefined
    ? schema.nodes.find(([key]) => key === schema.defaultKey)
    : undefined;
  const first = hit ?? schema.nodes[0];
  if (first === undefined) {
    throw ManipulationError.valueAtPathSegmentInvalid("no schema nodes found");
  }
  return first[1];
}

/** A fresh default value for a schema node: its `default` fn when present,
 * else a zero by shape. Used to create missing intermediate structures. */
export function defaultForNode(node: Schema | undefined): unknown {
  if (node === undefined) return null;
  const single = schemaAsNode(node);
  if (single.default !== undefined) return single.default();
  switch (single.shape.kind) {
    case "Struct":
    case "Map":
      return {};
    case "Sequence":
      return [];
    case "Scalar":
      return scalarZero(single.typeName);
    case "Enum":
      return single.shape.variants.some(([name]) => name === "None")
        ? { variant: "None" }
        : null;
    case "Indirection":
      return defaultForNode(single.shape.inner);
    case "Any":
      return null;
  }
}

/** The schema node at `path` when a schema was provided and resolves, else
 * undefined (the caller then passes the value through as-is — TS has no
 * static target type to coerce into without a schema). */
export function schemaNodeAt(path: PathSegment[], schema: Schema | undefined): SchemaNode | undefined {
  if (schema === undefined) return undefined;
  try {
    return schemaAtPath(path, schema).node;
  } catch {
    return undefined;
  }
}

/** Converts `value` through the schema node at `path` (when a schema resolves
 * there); schema failures are wrapped into ManipulationError. Without a
 * schema the value passes through unchanged. */
export function convertAtPath(value: unknown, path: PathSegment[], schema: Schema | undefined): unknown {
  const node = schemaNodeAt(path, schema);
  if (node === undefined) return value;
  try {
    return convert(value, node);
  } catch (err) {
    throw ManipulationError.conversionFailed(err instanceof Error ? err.message : String(err));
  }
}
