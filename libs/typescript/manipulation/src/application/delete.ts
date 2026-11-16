// `deleteAt`: remove the value(s) at a JSONPath from a source value (port of
// libs/rust/manipulation/src/application/delete.rs, itself ported from
// go-json's object.Delete / recursiveDelete / recursiveDescentDelete). Named
// `deleteAt` because `delete` is a reserved word in TS (the DynObject method
// is still `.delete(...)`, which is legal).
//
// Deletion is parent-side: a map key is removed, a sequence element spliced
// out (descending indices so they stay valid), a struct field reset to its
// zero value. Returns how many values were deleted. A root-only path resets a
// container source in place; a primitive root can only be reset through
// DynObject (which reassigns its held source).

import type { RecursiveDescentSegments } from "@dyn-rt/path";
import { asMap, asSequence, asStruct, mapContainsKey, mapDelete, mapKeys, seqDeleteIndex, structFieldNames } from "@dyn-rt/reflect";
import { reflectZero } from "@dyn-rt/reflect";
import { ManipulationError } from "../error.js";
import { PathCursor } from "../domain.js";
import {
  isEnumValue,
  isRootOnly,
  kindLabel,
  resolveIndex,
  selectorLabel,
  sliceIndices,
  structFieldBySelector,
  unionMemberIndices,
  unwrapAll,
} from "./helpers.js";

/** Deletes the value(s) in `source` at `segments`, returning how many were
 * deleted. */
export function deleteAt(source: unknown, segments: RecursiveDescentSegments): number {
  if (isRootOnly(segments)) {
    resetRootInPlace(source);
    return 1;
  }
  let count = 0;
  const cursor = PathCursor.start(segments.length);
  const onDelete = (): void => {
    count++;
  };
  if (cursor.recursive === cursor.lastRecursive) {
    linearDelete(source, segments, cursor, onDelete);
  } else {
    descentDelete(source, segments, cursor, onDelete);
  }
  return count;
}

/** Resets a container root to its zero in place. */
function resetRootInPlace(source: unknown): void {
  if (Array.isArray(source)) {
    source.length = 0;
    return;
  }
  const map = asMap(source);
  if (map !== undefined) {
    for (const key of Object.keys(map)) delete map[key];
    return;
  }
  const struct = asStruct(source);
  if (struct !== undefined) {
    for (const name of structFieldNames(struct)) {
      struct[name] = reflectZero(struct[name]);
    }
    return;
  }
  throw ManipulationError.valueAtPathSegmentInvalid(
    "cannot reset a primitive root in place; use DynObject.delete which reassigns its held source",
  );
}

type Count = () => void;

/** Linear traversal (no `..`); deletes at the current segment. */
function linearDelete(
  value: unknown,
  segments: RecursiveDescentSegments,
  cursor: PathCursor,
  count: Count,
): void {
  const current = unwrapAll(value);
  const segment = segments[cursor.recursive]![cursor.collection]!;
  const label = selectorLabel(segment.selector);
  const terminal = cursor.isTerminal(segments);

  if (current === null || current === undefined || isEnumValue(current)) {
    throw ManipulationError.valueAtPathSegmentInvalid(`value nil or invalid at segment ${label}`);
  }

  const map = asMap(current);
  if (map !== undefined) {
    switch (segment.selector.kind) {
      case "Root":
        advanceDelete(current, segments, cursor, count);
        return;
      case "Key": {
        const key = segment.selector.key;
        if (!mapContainsKey(map, key)) return; // deleting a non-existent key: no-op
        if (terminal) {
          mapDelete(map, key);
          count();
        } else {
          linearDelete(map[key], segments, cursor.nextInGroup(), count);
        }
        return;
      }
      case "Wildcard": {
        for (const key of [...mapKeys(map)]) {
          if (terminal) {
            mapDelete(map, key);
            count();
          } else {
            linearDelete(map[key], segments, cursor.nextInGroup(), count);
          }
        }
        return;
      }
      case "Union": {
        for (const member of segment.selector.members) {
          if (member.selector.kind !== "Key") continue;
          const key = member.selector.key;
          if (!mapContainsKey(map, key)) continue;
          if (terminal) {
            mapDelete(map, key);
            count();
          } else {
            linearDelete(map[key], segments, cursor.nextInGroup(), count);
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
        advanceDelete(current, segments, cursor, count);
        return;
      case "Index": {
        const idx = resolveIndex(segment.selector.index, seq.length);
        if (terminal) {
          seqDeleteIndex(seq, idx);
          count();
        } else {
          linearDelete(seq[idx], segments, cursor.nextInGroup(), count);
        }
        return;
      }
      case "Wildcard": {
        selectorDeleteIndices(seq, [...Array(seq.length).keys()], segments, cursor, terminal, count);
        return;
      }
      case "Union": {
        // delete highest index first so earlier indices stay valid
        const indices = [...new Set(unionMemberIndices(segment.selector.members, seq.length))].sort((a, b) => b - a);
        selectorDeleteIndices(seq, indices, segments, cursor, terminal, count);
        return;
      }
      case "Slice": {
        const indices = sliceIndices(segment.selector.slice, seq.length).sort((a, b) => b - a);
        selectorDeleteIndices(seq, indices, segments, cursor, terminal, count);
        return;
      }
      default:
        throw ManipulationError.pathSegmentInvalid(`segment ${label} unsupported for kind sequence`);
    }
  }

  const struct = asStruct(current);
  if (struct !== undefined) {
    switch (segment.selector.kind) {
      case "Root":
        advanceDelete(current, segments, cursor, count);
        return;
      case "Key":
      case "Index": {
        const field = structFieldBySelector(struct, segment.selector);
        if (!field.present) return; // missing field: no-op
        const name = segment.selector.kind === "Key" ? segment.selector.key : String(segment.selector.index);
        if (terminal) {
          struct[name] = reflectZero(field.value);
          count();
        } else {
          linearDelete(field.value, segments, cursor.nextInGroup(), count);
        }
        return;
      }
      case "Wildcard": {
        for (const name of structFieldNames(struct)) {
          if (terminal) {
            struct[name] = reflectZero(struct[name]);
            count();
          } else {
            linearDelete(struct[name], segments, cursor.nextInGroup(), count);
          }
        }
        return;
      }
      case "Union": {
        for (const member of segment.selector.members) {
          if (member.selector.kind !== "Key" && member.selector.kind !== "Index") continue;
          const field = structFieldBySelector(struct, member.selector);
          if (!field.present) continue;
          const name = member.selector.kind === "Key" ? member.selector.key : String(member.selector.index);
          if (terminal) {
            struct[name] = reflectZero(field.value);
            count();
          } else {
            linearDelete(field.value, segments, cursor.nextInGroup(), count);
          }
        }
        return;
      }
      default:
        throw ManipulationError.pathSegmentInvalid(`segment ${label} unsupported for kind struct`);
    }
  }

  throw ManipulationError.pathSegmentInvalid(
    `segment ${label} unsupported for kind ${kindLabel(current)}`,
  );
}

/** Deletes or recurses into each selected sequence index (indices are
 * pre-sorted descending so deletions don't shift later indices). */
function selectorDeleteIndices(
  seq: unknown[],
  indices: number[],
  segments: RecursiveDescentSegments,
  cursor: PathCursor,
  terminal: boolean,
  count: Count,
): void {
  for (const idx of indices) {
    if (terminal) {
      seqDeleteIndex(seq, idx);
      count();
    } else {
      linearDelete(seq[idx], segments, cursor.nextInGroup(), count);
    }
  }
}

/** Advances past the current segment (used for the root segment). */
function advanceDelete(
  value: unknown,
  segments: RecursiveDescentSegments,
  cursor: PathCursor,
  count: Count,
): void {
  if (cursor.isTerminal(segments)) {
    resetRootInPlace(value);
    count();
  } else if (cursor.isGroupEnd(segments)) {
    descentDelete(value, segments, cursor.nextGroup(), count);
  } else {
    linearDelete(value, segments, cursor.nextInGroup(), count);
  }
}

/** Recursive descent (`..`) delete. */
function descentDelete(
  value: unknown,
  segments: RecursiveDescentSegments,
  cursor: PathCursor,
  count: Count,
): void {
  const current = unwrapAll(value);
  const segment = segments[cursor.recursive]![cursor.collection]!;
  const label = selectorLabel(segment.selector);

  if (current === null || current === undefined || isEnumValue(current)) {
    throw ManipulationError.valueAtPathSegmentInvalid(
      `value nil or invalid at recursive descent ${label}`,
    );
  }
  if (segment.selector.kind === "Root") {
    linearDelete(current, segments, cursor, count);
    return;
  }
  if (segment.selector.kind !== "Key") {
    throw ManipulationError.pathSegmentInvalid(
      `recursive descent search segment ${label} is not a key`,
    );
  }
  const key = segment.selector.key;

  const map = asMap(current);
  if (map !== undefined) {
    for (const mapKey of [...mapKeys(map)]) {
      const matches = mapKey === key;
      if (matches && cursor.isTerminal(segments)) {
        // terminal match: delete the entry from the parent map
        mapDelete(map, mapKey);
        count();
        continue;
      }
      const entry = map[mapKey];
      if (matches) {
        if (cursor.isGroupEnd(segments)) {
          descentDelete(entry, segments, cursor.nextGroup(), count);
        } else {
          linearDelete(entry, segments, cursor.nextInGroup(), count);
        }
      }
      // keep descending into the entry at the same level
      descentDelete(map[mapKey], segments, cursor, count);
    }
    return;
  }

  const seq = asSequence(current);
  if (seq !== undefined) {
    for (let idx = seq.length - 1; idx >= 0; idx--) {
      descentDelete(seq[idx], segments, cursor, count);
    }
    return;
  }

  const struct = asStruct(current);
  if (struct !== undefined) {
    for (const name of structFieldNames(struct)) {
      const field = struct[name];
      if (name === key && cursor.isTerminal(segments)) {
        struct[name] = reflectZero(field);
        count();
        continue;
      }
      if (name === key) {
        if (cursor.isGroupEnd(segments)) {
          descentDelete(field, segments, cursor.nextGroup(), count);
        } else {
          linearDelete(field, segments, cursor.nextInGroup(), count);
        }
      }
      descentDelete(field, segments, cursor, count);
    }
    return;
  }

  throw ManipulationError.valueAtPathSegmentInvalid(
    `unsupported value at recursive descent search segment ${label} (kind ${kindLabel(current)})`,
  );
}
