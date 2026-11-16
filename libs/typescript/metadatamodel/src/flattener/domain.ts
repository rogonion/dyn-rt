// Flattener subdomain domain (port of
// libs/rust/metadatamodel/src/flattener/domain.rs): the 2D table types and
// the conversion tree.

import type { Value } from "@dyn-rt/shared";
import type { FieldColumnPosition } from "../fieldcolumns/index.js";

/** A single row of a flattened table. Every cell is an array (slice) for
 * uniformity; empty cells are empty arrays (go's `FlattenedRow` /
 * `DefaultEmptyColumn`). */
export type FlattenedRow = Value[];

/** A 2D linear collection — the result of flattening an object (go's
 * `FlattenedTable`). */
export type FlattenedTable = FlattenedRow[];

/** A node of the conversion tree (go's `RecursiveIndexTree`): the column
 * position of a field, plus child nodes for a nested group. */
export interface ConversionNode {
  /** The field/group this node reads. For a group node, `pathKey` is the
   * group's template; for a leaf, the field's (possibly separate-column)
   * position. */
  position: FieldColumnPosition;
  /** Child nodes when this node is a nested group. */
  children: ConversionNode[];
}

/** Re-exported row type alias for callers building tables by hand. */
export type Row = FlattenedRow;
