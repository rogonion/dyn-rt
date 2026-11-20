// Unflattener subdomain application use case (port of
// libs/rust/metadatamodel/src/unflattener/application.rs, itself ported
// from go-metadatamodel/unflattener): convert a 2D table back into a deeply
// nested collection in a destination, grouping rows into instances by
// signature (primary-key columns), and respecting field cardinality on
// write (a `single` field's cell array is unwrapped; more than one value
// errors unless the single-cell mode is `TakeFirst`).

import { DynObject } from "@dyn-rt/manipulation";
import type { RecursiveDescentSegments } from "@dyn-rt/path";
import type { Value } from "@dyn-rt/shared";
import { cardinalityOf, containsNestedGroups, dataPathModelAware, fieldsOf, isGroup, parseTemplate, pathKeyOf, prop, propsOf, readOrderOf } from "../core/index.js";
import { FIELDS_KEY, props, type MetadataModel } from "../core/domain.js";
import { MetadatamodelError } from "../error.js";
import { ColumnFields, extract, groupColumnIndexes, maxFlatColumns, type FieldColumnPosition } from "../fieldcolumns/index.js";
import type { FlattenedTable } from "../flattener/index.js";
import { GroupCollection, Signature } from "../groupindex/index.js";
import { DEFAULT_SINGLE_CELL_MODE, type GroupIndexTree, type SingleCellMode } from "./domain.js";

/** Converts a 2D table into a nested collection written to `destination`
 * (go's `Unflattener`). The destination root is always a collection. */
export class Unflattener {
  /** Extracted (and repositioned) column fields driving column order/skip. */
  readonly columnFields: ColumnFields;
  /** How to handle a `single`-cardinality field whose cell has >1 values. */
  singleCellMode: SingleCellMode = DEFAULT_SINGLE_CELL_MODE;
  private readonly tree: GroupIndexTree;
  private readonly index: GroupCollection = new GroupCollection();

  /** Creates an unflattener, extracting the default column fields from the
   * model (go's `NewUnflattener` + default extraction in `Unflatten`). */
  constructor(
    /** The metadata model describing the destination data. */
    public readonly model: MetadataModel,
    /** Where the reconstructed collection is written (a DynObject; attach a
     * schema to it to guide set creation/coercion). */
    public readonly destination: DynObject,
    columnFields?: ColumnFields,
  ) {
    if (columnFields !== undefined) {
      this.columnFields = columnFields;
    } else {
      const extracted = extract(model);
      extracted.reposition();
      extracted.updateIndexInRepositioned();
      this.columnFields = extracted;
    }
    this.tree = buildGroupIndexTree(model, "$", this.columnFields);
  }

  /** Creates an unflattener with caller-provided column fields. */
  static withColumnFields(model: MetadataModel, columnFields: ColumnFields, destination: DynObject): Unflattener {
    return new Unflattener(model, destination, columnFields);
  }

  /** Processes every row of `table` into the destination (go's `Unflatten`). */
  unflatten(table: FlattenedTable): void {
    for (const row of table) {
      recursiveConvert(this.tree, this.model, this.columnFields, this.destination, this.singleCellMode, this.index, [], row);
    }
  }
}

// ---------------------------------------------------------------------------
// group index tree (go's recursiveInitGroupIndexTree)
// ---------------------------------------------------------------------------

function buildGroupIndexTree(group: MetadataModel, groupPathKey: string, columnFields: ColumnFields): GroupIndexTree {
  const columnIndexes = groupColumnIndexes(columnFields, group);
  const tree: GroupIndexTree = {
    pathKey: groupPathKey,
    suffix: "",
    columnIndexes,
    children: [],
  };

  const order = readOrderOf(group);
  const fieldsObj = fieldsOf(group);

  for (const key of order) {
    const entry = fieldsObj[key];
    if (entry === undefined) {
      throw MetadatamodelError.invalidModel(`field ${JSON.stringify(key)} not found in ${JSON.stringify(FIELDS_KEY)}[0]`);
    }
    const fgProps = propsOf(entry);
    const pathKey = pathKeyOf(fgProps);

    if (isGroup(fgProps)) {
      const asSingle = prop(fgProps, props.EXTRACT_AS_SINGLE_FIELD) === true;
      if (asSingle) {
        continue; // processed as a plain field (its columns are already indexed)
      }
      const maxColumns = maxFlatColumns(fgProps);
      if (!containsNestedGroups(fgProps) && maxColumns > 0) {
        // group-as-separate-columns: no child node (go skips it)
        continue;
      }
      try {
        const child = buildGroupIndexTree(fgProps, pathKey, columnFields);
        child.suffix = key;
        tree.children.push(child);
      } catch (err) {
        // go's ErrNoGroupFields: skip empty nested groups
        if (!(err instanceof Error) || !err.message.includes("no group fields")) {
          throw err;
        }
      }
    }
  }
  return tree;
}

// ---------------------------------------------------------------------------
// recursive convert (go's recursiveConvert)
// ---------------------------------------------------------------------------

function recursiveConvert(
  tree: GroupIndexTree,
  model: MetadataModel,
  columnFields: ColumnFields,
  destination: DynObject,
  singleCellMode: SingleCellMode,
  parentCollection: GroupCollection,
  linearIndexes: number[],
  row: Value[],
): void {
  // 1. signature from primary key columns (else all columns)
  const signatureColumns = tree.columnIndexes.primary.length > 0 ? tree.columnIndexes.primary : tree.columnIndexes.all;
  const signature = new Signature().generateSignature(row, signatureColumns);

  // 2. get/create the instance
  const [instanceIndex, node] = parentCollection.getOrCreateInstance(signature, tree.pathKey);
  const currentPathIndexes = [...linearIndexes, instanceIndex];

  // 3. write this group's field columns
  const readOrder = columnFields.currentReadOrder();
  for (const colIndex of tree.columnIndexes.all) {
    const cell = row[colIndex];
    if (cell === undefined || cell === null) {
      continue;
    }
    const original = readOrder[colIndex];
    if (original === undefined) {
      continue;
    }
    const position = columnFields.readOrder[original];
    if (position === undefined) {
      continue;
    }
    const colField = columnFields.getByIndexInOriginalReadOrder(original);
    if (colField === undefined) {
      continue;
    }

    const target = leafTargetPath(position, model, currentPathIndexes);
    const valueToWrite = valueForCell(cell, colField.props, singleCellMode);
    // `set` converts into the existing value, which appends to sequences;
    // a whole-field (key-terminal) write must replace, so zero the target
    // first (delete on a class field zeroes it; indexed separate-column
    // slots already replace in place and are left alone).
    const lastGroup = target[target.length - 1];
    const lastSegment = lastGroup?.[lastGroup.length - 1];
    const keyTerminal = lastSegment !== undefined && lastSegment.selector.kind === "Key";
    if (keyTerminal) {
      // ignore: a missing target is fine on first write
      try {
        destination.delete(target);
      } catch {
        // first write: nothing to zero out
      }
    }
    try {
      destination.set(target, valueToWrite);
    } catch (err) {
      throw MetadatamodelError.manipulation(`write field failed: ${err instanceof Error ? err.message : String(err)}`);
    }
  }

  // 4. recurse into nested groups
  for (const child of tree.children) {
    const childCollection = node.getOrCreateGroup(child.suffix);
    recursiveConvert(child, model, columnFields, destination, singleCellMode, childCollection, currentPathIndexes, row);
  }
}

/** The target path a cell is written to: the destination root is a
 * collection, so the first path index addresses the root entry; the rest
 * address the nested groups. Separate-column leaves additionally index the
 * field's own array with their header index. */
function leafTargetPath(position: FieldColumnPosition, model: MetadataModel, pathIndexes: number[]): RecursiveDescentSegments {
  const rootIndex = pathIndexes[0];
  const rest = pathIndexes.slice(1);
  if (rootIndex === undefined) {
    throw MetadatamodelError.unflatten("no root instance index");
  }

  const indexes = [...rest];
  let indexLast: boolean;
  if (position.groupInSeparateColumns) {
    indexes.push(position.groupHeaderIndex);
    indexLast = true;
  } else if (position.fieldInSeparateColumns) {
    indexes.push(position.fieldHeaderIndex);
    indexLast = true;
  } else {
    indexLast = false;
  }

  const path = position.groupInSeparateColumns
    ? `${dataPathModelAware(position.groupParentPathKey, model, indexes, true)}.${position.fieldKeySuffix}`
    : dataPathModelAware(position.pathKey, model, indexes, indexLast);
  const segments = parseTemplate(path);
  const firstGroup = segments[0];
  if (firstGroup !== undefined) {
    // the destination root is a collection: address the root entry after `$`
    firstGroup.splice(1, 0, {
      accessStrategy: "Positional",
      selector: { kind: "Index", index: rootIndex },
    });
  }
  return segments;
}

/** Applies cardinality to a cell before writing: a `single` field's cell
 * array is unwrapped (erroring on >1 values unless TakeFirst); an `array`
 * field's cell is written as-is. */
function valueForCell(cell: Value, fieldProps: Value, singleCellMode: SingleCellMode): Value {
  const cardinality = cardinalityOf(fieldProps);
  if (cardinality === "single") {
    if (Array.isArray(cell)) {
      if (cell.length === 0) return null;
      if (cell.length === 1) return cell[0]!;
      if (singleCellMode === "TakeFirst") return cell[0]!;
      throw MetadatamodelError.unflatten(`single-cardinality field cell has ${cell.length} values`);
    }
    return cell;
  }
  return cell;
}
