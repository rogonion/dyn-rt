// Reflect application: the accessor use cases over native values (port of
// libs/rust/reflect's ReflectSequence / ReflectMap / ReflectStruct / scalar
// accessors, as plain functions). Each accessor is explicit and returns
// `undefined` on a kind mismatch instead of throwing, so callers can branch
// on the result (mirrors Rust's Option-returning trait methods).

import { isPlainObject, kindOf } from "@dyn-rt/shared";
import { reflectRef } from "./domain.js";

// ---------------------------------------------------------------------------
// sequence accessors (arrays)
// ---------------------------------------------------------------------------

export function asSequence(value: unknown): unknown[] | undefined {
  return Array.isArray(value) ? value : undefined;
}

export function seqLength(seq: unknown[]): number {
  return seq.length;
}

/** `undefined` when out of range. */
export function seqGet(seq: unknown[], index: number): unknown {
  return seq[index];
}

/** Sets index, growing the array with `undefined` when needed. */
export function seqSet(seq: unknown[], index: number, value: unknown): void {
  while (seq.length <= index) {
    seq.push(undefined);
  }
  seq[index] = value;
}

export function seqPush(seq: unknown[], value: unknown): void {
  seq.push(value);
}

export function seqDeleteIndex(seq: unknown[], index: number): void {
  if (index < seq.length) {
    seq.splice(index, 1);
  }
}

// ---------------------------------------------------------------------------
// map accessors (plain objects, string keys)
// ---------------------------------------------------------------------------

export function asMap(value: unknown): Record<string, unknown> | undefined {
  return isPlainObject(value) ? value : undefined;
}

export function mapGet(map: Record<string, unknown>, key: string): unknown {
  return map[key];
}

export function mapSet(map: Record<string, unknown>, key: string, value: unknown): void {
  map[key] = value;
}

/** Returns false when the key was absent. */
export function mapDelete(map: Record<string, unknown>, key: string): boolean {
  if (!Object.prototype.hasOwnProperty.call(map, key)) {
    return false;
  }
  delete map[key];
  return true;
}

export function mapContainsKey(map: Record<string, unknown>, key: string): boolean {
  return Object.prototype.hasOwnProperty.call(map, key);
}

export function mapKeys(map: Record<string, unknown>): string[] {
  return Object.keys(map);
}

export function mapLength(map: Record<string, unknown>): number {
  return Object.keys(map).length;
}

// ---------------------------------------------------------------------------
// class accessors (class instances; own enumerable string keys are fields)
// ---------------------------------------------------------------------------

export function asClass(value: unknown): Record<string, unknown> | undefined {
  if (reflectRef(value) !== "Class") return undefined;
  return value as Record<string, unknown>;
}

export function classFieldNames(value: object): string[] {
  return Object.keys(value);
}

export function classGet(obj: object, name: string): unknown {
  return (obj as Record<string, unknown>)[name];
}

export function classSet(obj: object, name: string, value: unknown): void {
  (obj as Record<string, unknown>)[name] = value;
}

// ---------------------------------------------------------------------------
// scalar accessors
// ---------------------------------------------------------------------------

export function scalarNumber(value: unknown): number | undefined {
  return typeof value === "number" ? value : undefined;
}

export function scalarBigint(value: unknown): bigint | undefined {
  return typeof value === "bigint" ? value : undefined;
}

export function scalarBool(value: unknown): boolean | undefined {
  return typeof value === "boolean" ? value : undefined;
}

export function scalarString(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

// ---------------------------------------------------------------------------
// whole-value operations
// ---------------------------------------------------------------------------

/** Deep clone preserving prototypes (class instances stay instances) and
 * Dates (port of `reflect_clone`). */
export function reflectClone<T>(value: T): T {
  if (value === null || typeof value !== "object") {
    return value;
  }
  if (value instanceof Date) {
    return new Date(value.getTime()) as T;
  }
  if (Array.isArray(value)) {
    return value.map((item) => reflectClone(item)) as T;
  }
  const clone: Record<string, unknown> = Object.create(Object.getPrototypeOf(value));
  for (const key of Object.keys(value)) {
    clone[key] = reflectClone((value as Record<string, unknown>)[key]);
  }
  return clone as T;
}

/** A zero value for the given value's shape (port of `reflect_zero`):
 * `0`/`0n`/`false`/`""` for scalars, `[]` for sequences, `{}` for maps, a
 * same-prototype instance with zeroed fields for classes, `null` for
 * null/undefined, `undefined` for opaque values. */
export function reflectZero(value: unknown): unknown {
  switch (reflectRef(value)) {
    case "Scalar":
      if (typeof value === "number") return 0;
      if (typeof value === "bigint") return 0n;
      if (typeof value === "boolean") return false;
      if (typeof value === "string") return "";
      if (value === undefined) return undefined;
      return null;
    case "Sequence":
      return [];
    case "Map":
      return {};
    case "Class": {
      const zero: Record<string, unknown> = Object.create(Object.getPrototypeOf(value));
      for (const key of Object.keys(value as object)) {
        zero[key] = reflectZero((value as Record<string, unknown>)[key]);
      }
      return zero;
    }
    default:
      return undefined;
  }
}

/** Deep-assign `source` into `target` (port of `reflect_apply` for map/class
 * targets): keys absent from `source` are removed, present keys are replaced
 * with deep clones. The target keeps its prototype. */
export function reflectApply(target: object, source: unknown): void {
  if (kindOf(source) !== "plain-object" && kindOf(source) !== "class-instance") {
    return; // scalar source into an object target: nothing sensible to assign
  }
  const targetMap = target as Record<string, unknown>;
  const sourceMap = source as Record<string, unknown>;
  for (const key of Object.keys(targetMap)) {
    if (!Object.prototype.hasOwnProperty.call(sourceMap, key)) {
      delete targetMap[key];
    }
  }
  for (const key of Object.keys(sourceMap)) {
    targetMap[key] = reflectClone(sourceMap[key]);
  }
}
