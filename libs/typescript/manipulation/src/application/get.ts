// `get`: retrieve the value(s) at a JSONPath from a source value (port of
// libs/rust/manipulation/src/application/get.rs, itself ported from go-json's
// object.Get / recursiveGet / selectorGetLoop / recursiveDescentGet).
//
// TS differences from Rust: values are plain references (`unknown`) routed by
// kindOf, and errors throw ManipulationError instead of returning Result.
// Returns every match as a list; an empty list means the path matched
// nothing.

import type { PathSegment, RecursiveDescentSegments } from "@dyn-rt/path";
import { asMap, asSequence, asClass, mapContainsKey, mapKeys, classFieldNames } from "@dyn-rt/reflect";
import { ManipulationError } from "../error.js";
import { PathCursor } from "../domain.js";
import {
  children,
  isRootOnly,
  kindLabel,
  resolveIndex,
  selectorLabel,
  sliceIndices,
  classFieldBySelector,
  unionMemberIndices,
  unwrapAll,
} from "./helpers.js";

/** Retrieves the value(s) in `source` at `segments`. An empty path or a lone
 * `$` yields the source itself (unwrapping a Some payload). */
export function get(source: unknown, segments: RecursiveDescentSegments): unknown[] {
  if (isRootOnly(segments)) {
    return [unwrapAll(source)];
  }
  const cursor = PathCursor.start(segments.length);
  if (cursor.recursive === cursor.lastRecursive) {
    return linearGet(source, segments, cursor);
  }
  return descentGet(source, segments, cursor);
}

/** Advances past the current segment into `value` (the value the current
 * segment resolved to). */
function advanceGet(value: unknown, segments: RecursiveDescentSegments, cursor: PathCursor): unknown[] {
  if (cursor.isTerminal(segments)) {
    return [value];
  }
  if (cursor.isGroupEnd(segments)) {
    return descentGet(value, segments, cursor.nextGroup());
  }
  return linearGet(value, segments, cursor.nextInGroup());
}

/** Linear traversal (no `..`): applies the current segment to `value`. */
function linearGet(value: unknown, segments: RecursiveDescentSegments, cursor: PathCursor): unknown[] {
  const current = unwrapAll(value);
  const segment = segments[cursor.recursive]![cursor.collection]!;
  const label = selectorLabel(segment.selector);

  switch (segment.selector.kind) {
    case "Root":
      return advanceGet(current, segments, cursor);
    case "Key": {
      const key = segment.selector.key;
      const map = asMap(current);
      if (map !== undefined) {
        if (!mapContainsKey(map, key)) return [];
        return advanceGet(map[key], segments, cursor);
      }
      const inst = asClass(current);
      if (inst !== undefined) {
        if (!Object.prototype.hasOwnProperty.call(inst, key)) return [];
        return advanceGet(inst[key], segments, cursor);
      }
      throw ManipulationError.pathSegmentInvalid(
        `key segment ${label} unsupported for kind ${kindLabel(current)}`,
      );
    }
    case "Index": {
      const seq = asSequence(current);
      if (seq !== undefined) {
        const idx = resolveIndex(segment.selector.index, seq.length);
        return advanceGet(seq[idx], segments, cursor);
      }
      // class positional field (the tuple analogue: $[1] == field "1")
      const inst = asClass(current);
      if (inst !== undefined) {
        const field = classFieldBySelector(inst, segment.selector);
        if (!field.present) return [];
        return advanceGet(field.value, segments, cursor);
      }
      throw ManipulationError.pathSegmentInvalid(
        `index segment ${label} unsupported for kind ${kindLabel(current)}`,
      );
    }
    case "Wildcard":
      return selectorGet(children(current), segments, cursor);
    case "Union":
      return selectorGet(unionSelect(current, segment.selector.members), segments, cursor);
    case "Slice": {
      const seq = asSequence(current);
      if (seq === undefined) {
        throw ManipulationError.pathSegmentInvalid(
          `slice segment ${label} unsupported for kind ${kindLabel(current)}`,
        );
      }
      const selected = sliceIndices(segment.selector.slice, seq.length).map((idx) => seq[idx]);
      return selectorGet(selected, segments, cursor);
    }
    case "Unrecognized":
      throw ManipulationError.pathSegmentInvalid(`unrecognized segment ${label}`);
  }
}

/** Applies a selector's selected values: terminal → all of them are results;
 * otherwise each continues the traversal and results are flattened. */
function selectorGet(selected: unknown[], segments: RecursiveDescentSegments, cursor: PathCursor): unknown[] {
  if (cursor.isTerminal(segments)) {
    return selected;
  }
  const results: unknown[] = [];
  for (const value of selected) {
    // a child that cannot continue the path simply yields no results
    // (go: an invalid recursiveGet result is skipped in the flatten loop)
    try {
      const child = cursor.isGroupEnd(segments)
        ? descentGet(value, segments, cursor.nextGroup())
        : linearGet(value, segments, cursor.nextInGroup());
      results.push(...child);
    } catch (err) {
      if (!(err instanceof ManipulationError)) throw err;
    }
  }
  return results;
}

/** Selects union members: keys against maps/classes, indices against sequences. */
function unionSelect(value: unknown, members: PathSegment[]): unknown[] {
  const current = unwrapAll(value);
  const map = asMap(current);
  if (map !== undefined) {
    const selected: unknown[] = [];
    for (const member of members) {
      if (member.selector.kind === "Key") {
        if (mapContainsKey(map, member.selector.key)) {
          selected.push(map[member.selector.key]);
        }
      } else if (member.selector.kind === "Wildcard") {
        for (const key of mapKeys(map)) selected.push(map[key]);
      }
    }
    return selected;
  }
  const seq = asSequence(current);
  if (seq !== undefined) {
    // index (incl. negative), slice and wildcard members all resolve to
    // concrete indices
    return unionMemberIndices(members, seq.length).map((idx) => seq[idx]);
  }
  const inst = asClass(current);
  if (inst !== undefined) {
    const selected: unknown[] = [];
    for (const member of members) {
      if (member.selector.kind === "Key" || member.selector.kind === "Index") {
        const field = classFieldBySelector(inst, member.selector);
        if (field.present) selected.push(field.value);
      } else if (member.selector.kind === "Wildcard") {
        for (const name of classFieldNames(inst)) selected.push(inst[name]);
      }
    }
    return selected;
  }
  throw ManipulationError.pathSegmentInvalid(
    `union segment unsupported for kind ${kindLabel(current)}`,
  );
}

/** Recursive descent (`..`): searches the current group's key segment at the
 * current level and all deeper levels. */
function descentGet(value: unknown, segments: RecursiveDescentSegments, cursor: PathCursor): unknown[] {
  const current = unwrapAll(value);
  const segment = segments[cursor.recursive]![cursor.collection]!;
  const label = selectorLabel(segment.selector);

  if (segment.selector.kind === "Root") {
    return linearGet(current, segments, cursor);
  }
  if (segment.selector.kind !== "Key") {
    throw ManipulationError.pathSegmentInvalid(
      `recursive descent search segment ${label} is not a key`,
    );
  }
  const key = segment.selector.key;

  const results: unknown[] = [];
  const map = asMap(current);
  if (map !== undefined) {
    for (const mapKey of mapKeys(map)) {
      const entry = map[mapKey];
      if (mapKey === key) {
        try {
          results.push(...advanceGet(entry, segments, cursor));
        } catch (err) {
          if (!(err instanceof ManipulationError)) throw err;
        }
      }
      // leaves (scalars) simply yield nothing at this level
      try {
        results.push(...descentGet(entry, segments, cursor));
      } catch (err) {
        if (!(err instanceof ManipulationError)) throw err;
      }
    }
    return results;
  }
  const seq = asSequence(current);
  if (seq !== undefined) {
    for (const element of seq) {
      try {
        results.push(...descentGet(element, segments, cursor));
      } catch (err) {
        if (!(err instanceof ManipulationError)) throw err;
      }
    }
    return results;
  }
  const inst = asClass(current);
  if (inst !== undefined) {
    for (const name of classFieldNames(inst)) {
      const field = inst[name];
      if (name === key) {
        try {
          results.push(...advanceGet(field, segments, cursor));
        } catch (err) {
          if (!(err instanceof ManipulationError)) throw err;
        }
      }
      try {
        results.push(...descentGet(field, segments, cursor));
      } catch (err) {
        if (!(err instanceof ManipulationError)) throw err;
      }
    }
    return results;
  }
  // scalar/unknown leaf: nothing to search at this level
  return results;
}
