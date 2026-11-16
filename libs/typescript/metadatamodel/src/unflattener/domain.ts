// Unflattener subdomain domain (port of
// libs/rust/metadatamodel/src/unflattener/domain.rs): the group index tree
// (the model->tree bridge over fieldcolumns) and the single-cell cardinality
// mode. The reusable instance/signature machinery itself lives in the
// `groupindex` subdomain and is re-exported here.

import type { GroupColumnIndexes } from "../fieldcolumns/index.js";
import type { GroupIndexNode } from "../groupindex/index.js";

export { GroupCollection, GroupIndexNode, Signature } from "../groupindex/index.js";

/** What to do when a `single`-cardinality field's cell holds more than one
 * value (the unflattener respects cardinality: a single value is unwrapped
 * from the cell array). */
export type SingleCellMode = "Error" | "TakeFirst";

/** The default single-cell mode: strict cardinality (error out). */
export const DEFAULT_SINGLE_CELL_MODE: SingleCellMode = "Error";

/** A node of the group index tree (go's `RecursiveGroupIndexTree`): which
 * columns belong to a group plus its nested group nodes. The runtime
 * instance structures (`GroupIndexNode`, `GroupCollection`, `Signature`)
 * live in the `groupindex` subdomain. */
export interface GroupIndexTree {
  /** The group's schema path key. */
  pathKey: string;
  /** The field-name suffix of this group within its parent (empty at root). */
  suffix: string;
  /** Which columns (unskipped current read-order indexes) belong to this group. */
  columnIndexes: GroupColumnIndexes;
  /** Nested group nodes. */
  children: GroupIndexTree[];
}

/** Re-export so callers can name the node type. */
export type Node = GroupIndexNode;
