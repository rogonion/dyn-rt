// `set`: create or update the value(s) at a JSONPath in a source value (port
// of libs/rust/manipulation/src/application/set.rs, itself ported from
// go-json's object.Set / recursiveSet / getDefaultValueAtPathSegment /
// recursiveDescentSet).
//
// TS differences from Rust: values are references, so mutation is direct (no
// clone-and-write-back); the value being set is converted through the schema
// node at the current path when a schema is provided (TS `convert` returns a
// new value which is then assigned); without a schema the value is assigned
// as-is (there is no static target type to coerce into). Returns how many
// values were set. A root-only path resets a container source in place;
// replacing a primitive root is only possible through DynObject (which holds
// the source in a field and can reassign it).

import type { PathSegment, RecursiveDescentSegments } from "@dyn-rt/path";
import { asMap, asSequence, asClass, mapContainsKey, mapKeys, mapSet, seqPush, seqSet, classFieldNames } from "@dyn-rt/reflect";
import type { Schema } from "@dyn-rt/schema";
import { ManipulationError } from "../error.js";
import { PathCursor } from "../domain.js";
import {
  convertAtPath,
  defaultForNode,
  isRootOnly,
  kindLabel,
  resolveSetIndex,
  schemaNodeAt,
  selectorLabel,
  sliceIndices,
  classFieldBySelector,
  unionMemberIndices,
  unwrapAll,
  isEnumValue,
} from "./helpers.js";

/** Sets `value` at `segments` in `source`, optionally guided by `schema` for
 * coercion and intermediate-structure creation. Returns how many values were
 * set. */
export function set(source: unknown, segments: RecursiveDescentSegments, value: unknown, schema?: Schema): number {
  if (isRootOnly(segments)) {
    const converted = convertAtPath(value, [], schema);
    applyRootInPlace(source, converted);
    return 1;
  }
  let count = 0;
  const path: PathSegment[] = [];
  const cursor = PathCursor.start(segments.length);
  if (cursor.recursive === cursor.lastRecursive) {
    linearSet(source, segments, cursor, value, schema, path, () => count++);
  } else {
    descentSet(source, segments, cursor, value, schema, path, () => count++);
  }
  return count;
}

/** Replaces a container root in place (arrays are spliced, objects deep-assigned).
 * A primitive root cannot be reassigned through a parameter — use DynObject. */
function applyRootInPlace(source: unknown, converted: unknown): void {
  if (Array.isArray(source)) {
    if (!Array.isArray(converted)) {
      throw ManipulationError.conversionFailed("cannot replace an array root with a non-array");
    }
    source.splice(0, source.length, ...converted);
    return;
  }
  if (typeof source === "object" && source !== null) {
    if (typeof converted !== "object" || converted === null) {
      throw ManipulationError.conversionFailed("cannot replace an object root with a scalar");
    }
    const target = source as Record<string, unknown>;
    for (const key of Object.keys(target)) delete target[key];
    Object.assign(target, converted);
    return;
  }
  throw ManipulationError.valueAtPathSegmentInvalid("cannot replace a primitive root in place; use DynObject.set which reassigns its held source");
}

// count is threaded through a small callback so the explicit parameter flow
// stays readable without a mutable counter class
type Count = () => void;

/** Linear traversal (no `..`); sets at the current segment. */
function linearSet(value: unknown, segments: RecursiveDescentSegments, cursor: PathCursor, valueToSet: unknown, schema: Schema | undefined, path: PathSegment[], count: Count): void {
  const current = unwrapAll(value);
  // a Some-enum payload is traversed through the wrapper object; an empty or
  // non-Some wrapper is an error here (like Rust's empty indirection/None)
  if (current === null || current === undefined || isEnumValue(current)) {
    const segment = segments[cursor.recursive]![cursor.collection]!;
    throw ManipulationError.valueAtPathSegmentInvalid(`value nil or invalid at segment ${selectorLabel(segment.selector)}`);
  }
  const segment = segments[cursor.recursive]![cursor.collection]!;
  const label = selectorLabel(segment.selector);
  const terminal = cursor.isTerminal(segments);

  const map = asMap(current);
  if (map !== undefined) {
    switch (segment.selector.kind) {
      case "Root":
        advanceSet(current, segments, cursor, valueToSet, schema, path, count);
        return;
      case "Key": {
        const key = segment.selector.key;
        ensureMapEntry(map, key, segment, path, schema);
        const entryPath = [...path, segment];
        if (terminal) {
          map[key] = convertAtPath(valueToSet, entryPath, schema);
          count();
        } else {
          path.push(segment);
          linearSet(map[key], segments, cursor.nextInGroup(), valueToSet, schema, path, count);
          path.pop();
        }
        return;
      }
      case "Wildcard": {
        for (const key of mapKeys(map)) {
          ensureMapEntry(map, key, segment, path, schema);
          const entryPath = [...path, segment];
          if (terminal) {
            map[key] = convertAtPath(valueToSet, entryPath, schema);
            count();
          } else {
            path.push(segment);
            linearSet(map[key], segments, cursor.nextInGroup(), valueToSet, schema, path, count);
            path.pop();
          }
        }
        return;
      }
      case "Union": {
        for (const member of segment.selector.members) {
          if (member.selector.kind !== "Key") continue;
          const key = member.selector.key;
          const memberSegment: PathSegment = {
            accessStrategy: member.accessStrategy,
            selector: { kind: "Key", key },
          };
          ensureMapEntry(map, key, memberSegment, path, schema);
          const entryPath = [...path, memberSegment];
          if (terminal) {
            map[key] = convertAtPath(valueToSet, entryPath, schema);
            count();
          } else {
            path.push(memberSegment);
            linearSet(map[key], segments, cursor.nextInGroup(), valueToSet, schema, path, count);
            path.pop();
          }
        }
        return;
      }
      default:
        throw ManipulationError.pathSegmentInvalid(`segment ${label} unsupported for kind map`);
    }
  }

  const seq = asSequence(current);
  if (seq !== undefined) {
    switch (segment.selector.kind) {
      case "Root":
        advanceSet(current, segments, cursor, valueToSet, schema, path, count);
        return;
      case "Index": {
        // unlike get, a set index may be past the end (expansion)
        setSequenceIndex(seq, resolveSetIndex(segment.selector.index, seq.length), segment, segments, cursor, valueToSet, schema, path, terminal, count);
        return;
      }
      case "Wildcard": {
        for (let idx = 0; idx < seq.length; idx++) {
          setSequenceIndex(seq, idx, segment, segments, cursor, valueToSet, schema, path, terminal, count);
        }
        return;
      }
      case "Union": {
        for (const idx of unionMemberIndices(segment.selector.members, seq.length)) {
          setSequenceIndex(seq, idx, segment, segments, cursor, valueToSet, schema, path, terminal, count);
        }
        return;
      }
      case "Slice": {
        for (const idx of sliceIndices(segment.selector.slice, seq.length)) {
          setSequenceIndex(seq, idx, segment, segments, cursor, valueToSet, schema, path, terminal, count);
        }
        return;
      }
      default:
        throw ManipulationError.pathSegmentInvalid(`segment ${label} unsupported for kind sequence`);
    }
  }

  const inst = asClass(current);
  if (inst !== undefined) {
    switch (segment.selector.kind) {
      case "Root":
        advanceSet(current, segments, cursor, valueToSet, schema, path, count);
        return;
      case "Key":
      case "Index": {
        const field = classFieldBySelector(inst, segment.selector);
        if (!field.present) {
          throw ManipulationError.valueAtPathSegmentInvalid(`class field ${label} not found`);
        }
        const name = segment.selector.kind === "Key" ? segment.selector.key : String(segment.selector.index);
        const fieldPath = [...path, segment];
        if (terminal) {
          inst[name] = convertAtPath(valueToSet, fieldPath, schema);
          count();
        } else {
          path.push(segment);
          linearSet(field.value, segments, cursor.nextInGroup(), valueToSet, schema, path, count);
          path.pop();
        }
        return;
      }
      case "Wildcard": {
        for (const name of classFieldNames(inst)) {
          const fieldPath = [...path, segment];
          if (terminal) {
            inst[name] = convertAtPath(valueToSet, fieldPath, schema);
            count();
          } else {
            path.push(segment);
            linearSet(inst[name], segments, cursor.nextInGroup(), valueToSet, schema, path, count);
            path.pop();
          }
        }
        return;
      }
      case "Union": {
        for (const member of segment.selector.members) {
          if (member.selector.kind !== "Key" && member.selector.kind !== "Index") continue;
          const field = classFieldBySelector(inst, member.selector);
          if (!field.present) continue;
          const name = member.selector.kind === "Key" ? member.selector.key : String(member.selector.index);
          const fieldPath = [...path, segment];
          if (terminal) {
            inst[name] = convertAtPath(valueToSet, fieldPath, schema);
            count();
          } else {
            path.push(segment);
            linearSet(field.value, segments, cursor.nextInGroup(), valueToSet, schema, path, count);
            path.pop();
          }
        }
        return;
      }
      default:
        throw ManipulationError.pathSegmentInvalid(`segment ${label} unsupported for kind class`);
    }
  }

  throw ManipulationError.pathSegmentInvalid(`segment ${label} unsupported for kind ${kindLabel(current)}`);
}

/** Ensures a map entry exists, creating it with a default when absent. */
function ensureMapEntry(map: Record<string, unknown>, key: string, segment: PathSegment, path: PathSegment[], schema: Schema | undefined): void {
  if (mapContainsKey(map, key)) return;
  const entryPath = [...path, segment];
  mapSet(map, key, defaultForNode(schemaNodeAt(entryPath, schema)));
}

/** Sets one sequence index: replaces in place, pushes at the end, or expands
 * with defaults up to the index (go's "set slice index with expansion"). */
function setSequenceIndex(
  seq: unknown[],
  idx: number,
  segment: PathSegment,
  segments: RecursiveDescentSegments,
  cursor: PathCursor,
  valueToSet: unknown,
  schema: Schema | undefined,
  path: PathSegment[],
  terminal: boolean,
  count: Count,
): void {
  const indexPath = [...path, segment];
  // expand with defaults when the index is past the end
  while (seq.length <= idx) {
    seqPush(seq, defaultForNode(schemaNodeAt(indexPath, schema)));
  }
  if (terminal) {
    seqSet(seq, idx, convertAtPath(valueToSet, indexPath, schema));
    count();
  } else {
    path.push(segment);
    linearSet(seq[idx], segments, cursor.nextInGroup(), valueToSet, schema, path, count);
    path.pop();
  }
}

/** Advances past the current (root) segment. */
function advanceSet(value: unknown, segments: RecursiveDescentSegments, cursor: PathCursor, valueToSet: unknown, schema: Schema | undefined, path: PathSegment[], count: Count): void {
  if (cursor.isTerminal(segments)) {
    applyRootInPlace(value, convertAtPath(valueToSet, path, schema));
    count();
  } else if (cursor.isGroupEnd(segments)) {
    descentSet(value, segments, cursor.nextGroup(), valueToSet, schema, path, count);
  } else {
    linearSet(value, segments, cursor.nextInGroup(), valueToSet, schema, path, count);
  }
}

/** Recursive descent (`..`) set: applies the rest of the path at every level
 * where the search key matches, and keeps descending into all children. */
function descentSet(value: unknown, segments: RecursiveDescentSegments, cursor: PathCursor, valueToSet: unknown, schema: Schema | undefined, path: PathSegment[], count: Count): void {
  const current = unwrapAll(value);
  const segment = segments[cursor.recursive]![cursor.collection]!;
  const label = selectorLabel(segment.selector);

  if (current === null || current === undefined || isEnumValue(current)) {
    throw ManipulationError.valueAtPathSegmentInvalid(`value nil or invalid at recursive descent ${label}`);
  }
  if (segment.selector.kind === "Root") {
    linearSet(current, segments, cursor, valueToSet, schema, path, count);
    return;
  }
  if (segment.selector.kind !== "Key") {
    throw ManipulationError.pathSegmentInvalid(`recursive descent search segment ${label} is not a key`);
  }
  const key = segment.selector.key;

  const map = asMap(current);
  if (map !== undefined) {
    for (const mapKey of [...mapKeys(map)]) {
      const matches = mapKey === key;
      if (matches) {
        ensureMapEntry(map, mapKey, segment, path, schema);
      }
      const entry = map[mapKey];
      if (matches) {
        path.push(segment);
        if (cursor.isTerminal(segments)) {
          map[mapKey] = convertAtPath(valueToSet, path, schema);
          count();
        } else if (cursor.isGroupEnd(segments)) {
          descentSet(entry, segments, cursor.nextGroup(), valueToSet, schema, path, count);
        } else {
          linearSet(entry, segments, cursor.nextInGroup(), valueToSet, schema, path, count);
        }
        path.pop();
      }
      // keep descending into the entry at the same level
      if (!matches || !cursor.isTerminal(segments)) {
        descentSet(map[mapKey], segments, cursor, valueToSet, schema, path, count);
      }
    }
    return;
  }

  const seq = asSequence(current);
  if (seq !== undefined) {
    for (const element of seq) {
      descentSet(element, segments, cursor, valueToSet, schema, path, count);
    }
    return;
  }

  const inst = asClass(current);
  if (inst !== undefined) {
    for (const name of classFieldNames(inst)) {
      const field = inst[name];
      if (name === key) {
        path.push(segment);
        if (cursor.isTerminal(segments)) {
          inst[name] = convertAtPath(valueToSet, path, schema);
          count();
        } else if (cursor.isGroupEnd(segments)) {
          descentSet(field, segments, cursor.nextGroup(), valueToSet, schema, path, count);
        } else {
          linearSet(field, segments, cursor.nextInGroup(), valueToSet, schema, path, count);
        }
        path.pop();
      }
      descentSet(field, segments, cursor, valueToSet, schema, path, count);
    }
    return;
  }

  throw ManipulationError.valueAtPathSegmentInvalid(`unsupported value at recursive descent search segment ${label} (kind ${kindLabel(current)})`);
}
