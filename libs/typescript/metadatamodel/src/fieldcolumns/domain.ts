// fieldcolumns domain (port of
// libs/rust/metadatamodel/src/fieldcolumns/domain.rs): the metadata model's
// fields as columns of a flat 2D table (port of go-metadatamodel/fieldcolumns
// core.go).

import type { Schema } from "@dyn-rt/schema";
import type { Value } from "@dyn-rt/shared";

/** Where a column sits: the field's `group:path_key`, optionally expanded
 * into separate columns (one per value index) for fields/groups with
 * `view:values_in_separate_columns`. */
export interface FieldColumnPosition {
  /** The field's `group:path_key` template. */
  pathKey: string;
  fieldInSeparateColumns: boolean;
  fieldHeaderIndex: number;
  groupInSeparateColumns: boolean;
  groupHeaderIndex: number;
  groupParentPathKey: string;
  fieldKeySuffix: string;
  /** `view:position_before`: insert before the destination instead of after. */
  positionBefore: boolean;
  /** Original read-order index of the source column (set when the position
   * is recorded as a reposition instruction). */
  sourceIndex: number;
}

/** A position with defaults, overridden per use site. */
export function fieldColumnPosition(pathKey: string, overrides: Partial<FieldColumnPosition> = {}): FieldColumnPosition {
  return {
    pathKey,
    fieldInSeparateColumns: false,
    fieldHeaderIndex: 0,
    groupInSeparateColumns: false,
    groupHeaderIndex: 0,
    groupParentPathKey: "",
    fieldKeySuffix: "",
    positionBefore: false,
    sourceIndex: 0,
    ...overrides,
  };
}

/** The key identifying this column in ColumnFields.fields: for
 * group-separate-columns `parent[i].suffix`, for field-separate-columns
 * `path_key[i]`, else the plain `path_key`. */
export function columnKeyOf(position: FieldColumnPosition): string {
  if (position.groupInSeparateColumns) {
    return `${position.groupParentPathKey}[${position.groupHeaderIndex}].${position.fieldKeySuffix}`;
  }
  if (position.fieldInSeparateColumns) {
    return `${position.pathKey}[${position.fieldHeaderIndex}]`;
  }
  return position.pathKey;
}

/** One extracted column: the field's props (deep-copied and merged with any
 * matched parent props) plus bookkeeping. */
export interface ColumnField {
  /** The field/group props object (owned copy — the model is not mutated). */
  props: Value;
  /** The field's schema, when an extraction schema was provided. */
  schema?: Schema;
  /** Index in the original read order (extraction order). */
  indexInReadOrder: number;
  /** Index after repositioning (see reposition()). */
  indexInRepositioned: number;
  /** Set by skip(). */
  skip: boolean;
}

/** The extraction result: fields as columns in a flat 2D table. */
export class ColumnFields {
  /** Column key -> field, in insertion order. */
  fields: [string, ColumnField][] = [];
  /** Read order of the columns (positions, incl. separate-column expansion). */
  readOrder: FieldColumnPosition[] = [];
  /** Current order as indexes into `readOrder`; mutated by reposition(). */
  current: number[] = [];
  /** Reposition instructions (from `view:column_position` props). */
  repositionInstructions: FieldColumnPosition[] = [];

  /** Looks a column up by its column key. */
  getByKey(columnKey: string): ColumnField | undefined {
    return this.fields.find(([key]) => key === columnKey)?.[1];
  }

  /** Looks a column up by index into the current (possibly repositioned)
   * read order. */
  getByIndexInCurrentReadOrder(index: number): ColumnField | undefined {
    const original = this.current[index];
    if (original === undefined) return undefined;
    return this.getByIndexInOriginalReadOrder(original);
  }

  /** Looks a column up by index into the original read order. */
  getByIndexInOriginalReadOrder(index: number): ColumnField | undefined {
    const position = this.readOrder[index];
    if (position === undefined) return undefined;
    return this.getByKey(columnKeyOf(position));
  }

  /** Current read order with skipped fields removed. */
  currentReadOrder(): number[] {
    return this.current.filter((original) => {
      const field = this.getByIndexInOriginalReadOrder(original);
      return field === undefined || !field.skip;
    });
  }

  /** Applies the recorded reposition instructions to `current` (go's
   * Reposition): each instruction moves `sourceIndex` (an original read-order
   * index) before/after the destination column. */
  reposition(): void {
    const total = this.current.length;
    for (const instruction of [...this.repositionInstructions]) {
      const destinationOriginal = this.readOrder.findIndex((p) => columnKeyOf(p) === columnKeyOf(instruction));
      if (destinationOriginal === -1) continue;
      const destinationCurrent = this.current.indexOf(destinationOriginal);
      if (destinationCurrent === -1) continue;
      const sourceCurrent = this.current.indexOf(instruction.sourceIndex);
      if (sourceCurrent === -1) continue;
      const destination = instruction.positionBefore || destinationCurrent >= total - 1 ? destinationCurrent : destinationCurrent + 1;
      if (destination !== sourceCurrent) {
        const source = this.current.splice(sourceCurrent, 1)[0]!;
        this.current.splice(Math.min(destination, this.current.length), 0, source);
      }
    }
  }

  /** Position of `originalIndex` within the unskipped current read order
   * (go's `IndexInUnskippedColumnFields`). */
  indexInUnskipped(originalIndex: number): number | undefined {
    const position = this.currentReadOrder().indexOf(originalIndex);
    return position === -1 ? undefined : position;
  }

  /** Updates each column's `indexInRepositioned` from `current`. */
  updateIndexInRepositioned(): void {
    for (let readIndex = 0; readIndex < this.current.length; readIndex++) {
      const original = this.current[readIndex]!;
      const entry = this.fields.find(([, field]) => field.indexInReadOrder === original);
      if (entry !== undefined) {
        entry[1].indexInRepositioned = readIndex;
      }
    }
  }

  /** Marks columns to skip: a valid skip-match skips the column; a valid
   * add-match keeps it only when it matches. */
  skip(skipMatch?: PropMatchLike, addMatch?: PropMatchLike): void {
    for (const [, field] of this.fields) {
      field.skip = false;
      if (skipMatch !== undefined && skipMatch.isValid() && skipMatch.firstMatch(field.props)) {
        field.skip = true;
      }
      if (addMatch !== undefined && addMatch.isValid() && !addMatch.firstMatch(field.props)) {
        field.skip = true;
      }
    }
  }
}

/** Structural type of the PropMatch used by skip (defined in application). */
export interface PropMatchLike {
  isValid(): boolean;
  firstMatch(props: Value): boolean;
}
