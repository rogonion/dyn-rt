// Domain types for walking a parsed JSONPath (port of
// libs/rust/manipulation/src/domain.rs): the cursor tracking which
// recursive-descent group and which segment within the group is current
// (port of go-json's `internal.PathSegmentsIndexes`).

import type { RecursiveDescentSegments } from "@dyn-rt/path";

/** Position while walking a RecursiveDescentSegments. */
export class PathCursor {
  constructor(
    /** Index of the current recursive-descent group (`..`-separated part). */
    public recursive: number,
    /** Index of the current segment within the current group. */
    public collection: number,
    /** Index of the last recursive-descent group. */
    public lastRecursive: number,
  ) {}

  /** A cursor starting at the first group / first segment. */
  static start(groupCount: number): PathCursor {
    return new PathCursor(0, 0, Math.max(groupCount - 1, 0));
  }

  /** Whether the cursor sits on the very last segment of the whole path. */
  isTerminal(segments: RecursiveDescentSegments): boolean {
    const group = segments[this.recursive]!;
    return this.collection === Math.max(group.length - 1, 0) && this.recursive === this.lastRecursive;
  }

  /** Whether the cursor sits on the last segment of its group (so the next
   * group begins at a `..` boundary). */
  isGroupEnd(segments: RecursiveDescentSegments): boolean {
    const group = segments[this.recursive]!;
    return this.collection === Math.max(group.length - 1, 0);
  }

  /** Cursor advanced by one segment within the current group. */
  nextInGroup(): PathCursor {
    return new PathCursor(this.recursive, this.collection + 1, this.lastRecursive);
  }

  /** Cursor moved to the first segment of the next recursive-descent group. */
  nextGroup(): PathCursor {
    return new PathCursor(this.recursive + 1, 0, this.lastRecursive);
  }
}
