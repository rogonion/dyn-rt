// Flattener subdomain application use case (port of
// libs/rust/metadatamodel/src/flattener/application.rs, itself ported from
// go-metadatamodel/flattener): convert a deeply nested mix of collections
// (e.g. an array of objects) into a 2D table, using the fieldcolumns
// extraction of the metadata model for column order/skip, and optionally
// write the table to a destination 2D collection (`writeToDestination`).

import { DynObject, isEnumValue } from "@dyn-rt/manipulation";
import type { RecursiveDescentSegments } from "@dyn-rt/path";
import { classFieldNames, classGet } from "@dyn-rt/reflect";
import { isPlainObject, type JsonObject, type Value } from "@dyn-rt/shared";
import {
  containsNestedGroups,
  dataPathModelAware,
  fieldsOf,
  isGroup,
  parseTemplate,
  pathKeyOf,
  prop,
  propsOf,
  readOrderOf,
} from "../core/index.js";
import { FIELDS_KEY, props, type MetadataModel } from "../core/domain.js";
import { MetadatamodelError } from "../error.js";
import {
  ColumnFields,
  columnKeyOf,
  extract,
  fieldColumnPosition,
  maxFlatColumns,
  type FieldColumnPosition,
} from "../fieldcolumns/index.js";
import { type ConversionNode, type FlattenedRow, type FlattenedTable } from "./domain.js";

/** Converts objects into a 2D table (go's `Flattener`). */
export class Flattener {
  /** Extracted (and repositioned) column fields driving column order/skip. */
  readonly columnFields: ColumnFields;
  private readonly tree: ConversionNode;
  private resultRows: FlattenedTable = [];

  /** Creates a flattener, extracting the default column fields from the
   * model (go's `NewFlattener` + default extraction in `Flatten`). */
  constructor(
    /** The metadata model describing the source data. */
    public readonly model: MetadataModel,
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
    this.tree = buildConversionTree(model, "$", this.columnFields, true);
  }

  /** Creates a flattener with caller-provided column fields (go's
   * `WithColumnFields`). */
  static withColumnFields(model: MetadataModel, columnFields: ColumnFields): Flattener {
    return new Flattener(model, columnFields);
  }

  /** The current flattened table (go's `GetResult`). */
  result(): FlattenedTable {
    return this.resultRows;
  }

  /** Clears the current result (go's `Reset`). */
  reset(): void {
    this.resultRows = [];
  }

  /** Flattens `source` and appends the rows to the result. When the source
   * root is a sequence, each element is flattened individually (go's
   * `Flatten`). */
  flatten(source: DynObject): void {
    const src = source.getSource();
    if (Array.isArray(src)) {
      for (const element of src) {
        const rows = recursiveConvert(this.tree, this.model, element, [], []);
        this.resultRows.push(...rows);
      }
    } else {
      const rows = recursiveConvert(this.tree, this.model, src, [], []);
      this.resultRows.push(...rows);
    }
  }

  /** Writes the current result to a destination 2D collection at
   * `$[row][col]`, honoring the unskipped current column read order (go's
   * `WriteToDestination`). The destination's attached schema (if any)
   * guides coercion/creation. */
  writeToDestination(destination: DynObject): void {
    const readOrder = this.columnFields.currentReadOrder();
    for (let rowIndex = 0; rowIndex < this.resultRows.length; rowIndex++) {
      const row = this.resultRows[rowIndex]!;
      for (let destCol = 0; destCol < readOrder.length; destCol++) {
        const sourceCol = readOrder[destCol]!;
        const cell = row[sourceCol];
        if (cell === undefined) {
          throw MetadatamodelError.flatten(
            `source column index ${sourceCol} out of bounds (row ${rowIndex}, row length ${row.length})`,
          );
        }
        const target = parseTemplate(`$[${rowIndex}][${destCol}]`);
        try {
          destination.set(target, cell);
        } catch (err) {
          throw MetadatamodelError.manipulation(
            `write failed at row ${rowIndex} col ${destCol}: ${err instanceof Error ? err.message : String(err)}`,
          );
        }
      }
    }
  }
}

// ---------------------------------------------------------------------------
// conversion tree (go's recursiveInitRecursiveIndexTree)
// ---------------------------------------------------------------------------

/** Builds the conversion tree for `group` (root when `isRoot`). A group
 * with no usable fields is an error at the root and skipped when nested
 * (go's `ErrNoGroupFields`). */
function buildConversionTree(
  group: MetadataModel,
  groupPathKey: string,
  columnFields: ColumnFields,
  isRoot: boolean,
): ConversionNode {
  const node: ConversionNode = { position: fieldColumnPosition(groupPathKey), children: [] };

  const order = readOrderOf(group);
  const fieldsObj = fieldsOf(group);

  for (const key of order) {
    const entry = fieldsObj[key];
    if (entry === undefined) {
      throw MetadatamodelError.invalidModel(
        `field ${JSON.stringify(key)} not found in ${JSON.stringify(FIELDS_KEY)}[0]`,
      );
    }
    const fgProps = propsOf(entry);
    const pathKey = pathKeyOf(fgProps);

    if (isGroup(fgProps)) {
      const asSingle = prop(fgProps, props.EXTRACT_AS_SINGLE_FIELD) === true;
      const maxColumns = maxFlatColumns(fgProps);
      if (!asSingle && maxColumns > 0 && !containsNestedGroups(fgProps)) {
        // group-as-separate-columns: one leaf per nested field per column
        const nestedOrder = readOrderOf(fgProps);
        const nestedObj = fieldsOf(fgProps);
        for (let columnIndex = 0; columnIndex < maxColumns; columnIndex++) {
          for (const nestedSuffix of nestedOrder) {
            const nestedEntry = nestedObj[nestedSuffix];
            if (nestedEntry === undefined) {
              throw MetadatamodelError.invalidModel(
                `nested field ${JSON.stringify(nestedSuffix)} not found`,
              );
            }
            const nestedKey = pathKeyOf(propsOf(nestedEntry));
            const position = fieldColumnPosition(nestedKey, {
              groupInSeparateColumns: true,
              groupHeaderIndex: columnIndex,
              groupParentPathKey: pathKey,
              fieldKeySuffix: nestedSuffix,
            });
            pushLeaf(node, position, columnFields);
          }
        }
        continue;
      }

      // nested group: a child node with its own children
      try {
        const child = buildConversionTree(fgProps, pathKey, columnFields, false);
        node.children.push(child);
      } catch (err) {
        // go's ErrNoGroupFields: skip empty nested groups (but not at root)
        if (isRoot || !(err instanceof Error) || !err.message.includes("no group fields")) {
          throw err;
        }
      }
      continue;
    }

    // plain field
    const maxColumns = maxFlatColumns(fgProps);
    if (maxColumns > 0) {
      for (let columnIndex = 0; columnIndex < maxColumns; columnIndex++) {
        const position = fieldColumnPosition(pathKey, {
          fieldInSeparateColumns: true,
          fieldHeaderIndex: columnIndex,
        });
        pushLeaf(node, position, columnFields);
      }
    } else {
      pushLeaf(node, fieldColumnPosition(pathKey), columnFields);
    }
  }

  if (node.children.length === 0) {
    throw MetadatamodelError.flatten("no group fields to extract found");
  }
  return node;
}

/** Adds a leaf child when the position maps to an unskipped column. */
function pushLeaf(
  node: ConversionNode,
  position: FieldColumnPosition,
  columnFields: ColumnFields,
): void {
  const field = columnFields.getByKey(columnKeyOf(position));
  if (field !== undefined && !field.skip) {
    node.children.push({ position, children: [] });
  }
}

// ---------------------------------------------------------------------------
// recursive convert (go's recursiveConvert)
// ---------------------------------------------------------------------------

/** The data path a leaf reads: `linearIndexes` address the enclosing group
 * arrays; separate-column leaves additionally index the field's own array
 * with their header index; plain leaves read the field's whole value.
 * Cardinality is model-aware: `single` fields/groups drop their `[*]`. */
function leafDataPath(
  position: FieldColumnPosition,
  model: MetadataModel,
  linearIndexes: number[],
): RecursiveDescentSegments {
  if (position.groupInSeparateColumns) {
    // parent group array indexed by the header index, then the bare suffix
    const indexes = [...linearIndexes, position.groupHeaderIndex];
    const parent = dataPathModelAware(position.groupParentPathKey, model, indexes, true);
    return parseTemplate(`${parent}.${position.fieldKeySuffix}`);
  }
  if (position.fieldInSeparateColumns) {
    // the field's own array indexed by the column header index
    const indexes = [...linearIndexes, position.fieldHeaderIndex];
    return parseTemplate(dataPathModelAware(position.pathKey, model, indexes, true));
  }
  // plain leaf: read the whole value (last wildcard dropped)
  return parseTemplate(dataPathModelAware(position.pathKey, model, linearIndexes, false));
}

/** The data path a group node branches on: the whole group value (last
 * wildcard dropped), with enclosing group arrays indexed. */
function groupDataPath(
  groupTemplate: string,
  model: MetadataModel,
  linearIndexes: number[],
): RecursiveDescentSegments {
  return parseTemplate(dataPathModelAware(groupTemplate, model, linearIndexes, false));
}

/** Converts `source` into rows according to `node`, branching per array
 * element of nested groups (Cartesian product across group arrays). */
function recursiveConvert(
  node: ConversionNode,
  model: MetadataModel,
  source: unknown,
  linearIndexes: number[],
  incomingRows: FlattenedTable,
): FlattenedTable {
  let currentRows = incomingRows;
  if (currentRows.length === 0) {
    currentRows = [[]];
  }

  for (const child of node.children) {
    // resolve the child's own value at the current position
    const childSegments =
      child.children.length === 0
        ? leafDataPath(child.position, model, linearIndexes)
        : // group node: read the WHOLE group value (trailing [*] trimmed);
          // branching per array element happens below on the actual kind
          groupDataPath(child.position.pathKey, model, linearIndexes);
    let values: unknown[];
    try {
      values = DynObject.new(source).get(childSegments);
    } catch (err) {
      throw MetadatamodelError.manipulation(err instanceof Error ? err.message : String(err));
    }
    const fieldData = values[0];

    if (child.children.length > 0) {
      // nested group: branch per array element
      if (Array.isArray(fieldData)) {
        let collected: FlattenedTable = [];
        for (let i = 0; i < fieldData.length; i++) {
          const indexes = [...linearIndexes, i];
          // each branch gets its own copy of the rows so far (go's copyTable)
          const branchRows = recursiveConvert(child, model, source, indexes, copyTable(currentRows));
          collected.push(...branchRows);
        }
        currentRows = collected;
      } else {
        const indexes = [...linearIndexes, 0];
        currentRows = recursiveConvert(child, model, source, indexes, currentRows);
      }
      continue;
    }

    // leaf: append the cell to every current row
    const cell = cellValue(fieldData);
    for (const row of currentRows) {
      row.push(cell);
    }
  }

  return currentRows;
}

/** Normalizes a found value into a cell: sequences stay whole, scalars are
 * wrapped in a single-element array, missing values become an empty array
 * (go's `getCellValueFromFieldData` / `DefaultEmptyColumn`). */
function cellValue(fieldData: unknown): Value {
  if (fieldData === undefined) {
    return [];
  }
  if (Array.isArray(fieldData)) {
    return toValue(fieldData);
  }
  return [toValue(fieldData)];
}

/** Converts a runtime value into a [`Value`] (for table cells). */
export function toValue(value: unknown): Value {
  if (value === null || value === undefined) return null;
  if (typeof value === "boolean" || typeof value === "number" || typeof value === "string") {
    return value;
  }
  if (typeof value === "bigint") return Number(value);
  if (Array.isArray(value)) {
    return value.map((element) => toValue(element));
  }
  // Some-enum shape: { variant, payload? }
  if (isEnumValue(value)) {
    if (value.payload !== undefined) {
      return { variant: value.variant, payload: toValue(value.payload) };
    }
    return value.variant;
  }
  if (isPlainObject(value)) {
    const out: JsonObject = {};
    for (const [key, entry] of Object.entries(value)) {
      out[key] = toValue(entry);
    }
    return out;
  }
  if (typeof value === "object") {
    // class instance: read its fields into a plain object
    const out: JsonObject = {};
    for (const name of classFieldNames(value)) {
      out[name] = toValue(classGet(value, name));
    }
    return out;
  }
  // opaque leaves (Date, functions, ...): null
  return null;
}

/** Rows are cloned when branching (go's copyTable): each branch gets its own
 * copy so appending cells in one branch does not affect the others. */
export function copyTable(source: FlattenedTable): FlattenedTable {
  return source.map((row: FlattenedRow) => row.map((cell) => cloneValue(cell)));
}

function cloneValue(value: Value): Value {
  return JSON.parse(JSON.stringify(value)) as Value;
}
