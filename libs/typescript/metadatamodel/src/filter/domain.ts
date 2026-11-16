// Filter subdomain domain (port of
// libs/rust/metadatamodel/src/filter/domain.rs): the query-condition
// vocabulary, the logical operator, and the condition-processor contract
// (go's `FilterContext` / `ConditionTrue`).

import type { Value } from "@dyn-rt/shared";

/** Keys inside a filter condition's data object (go's `FilterCondition*`). */
export const conditionProps = {
  VALUE: "Value",
  VALUES: "Values",
  DATE_TIME_FORMAT: "DateTimeFormat",
  CASE_INSENSITIVE: "CaseInsensitive",
  ASSUMED_FIELD_TYPE: "AssumedFieldType",
} as const;

/** Keys of a query condition object (go's `QueryCondition*` / `QuerySection*`). */
export const query = {
  TYPE: "Type",
  NEGATE: "Negate",
  VALUE: "Value",
  LOGICAL_OPERATOR: "LogicalOperator",
  TYPE_LOGICAL_OPERATOR: "LogicalOperator",
  TYPE_FIELD_GROUP: "FieldGroup",
  AND: "And",
  OR: "Or",
} as const;

/** The filter condition names — keys that map to condition processors
 * (go's `FilterCondition*`). */
export const condition = {
  NO_OF_ENTRIES_GREATER_THAN: "NoOfEntriesGreaterThan",
  NO_OF_ENTRIES_LESS_THAN: "NoOfEntriesLessThan",
  NO_OF_ENTRIES_EQUAL_TO: "NoOfEntriesEqualTo",
  GREATER_THAN: "GreaterThan",
  LESS_THAN: "LessThan",
  EQUAL_TO: "EqualTo",
  BEGINS_WITH: "BeginsWith",
  ENDS_WITH: "EndsWith",
  CONTAINS: "Contains",
} as const;

/** Assumed field types in a filter condition (go's `core.FieldType*`). */
export const fieldType = {
  TEXT: "Text",
  NUMBER: "Number",
  TIMESTAMP: "Timestamp",
  ANY: "Any",
} as const;

/** Datetime format granularity keys (go's `core.FieldDatetimeFormat*`). */
export const datetimeFormat = {
  YYYYMMDDHHMM: "yyyy-mm-dd hh:mm",
  YYYYMMDD: "yyyy-mm-dd",
  YYYYMM: "yyyy-mm",
  YYYY: "yyyy",
  MM: "mm",
  HHMM: "hh:mm",
} as const;

/** The logical operator combining sub-conditions. */
export type LogicalOperator = "And" | "Or";

export function parseLogicalOperator(value: string): LogicalOperator | undefined {
  if (value === query.AND) return "And";
  if (value === query.OR) return "Or";
  return undefined;
}

/** Context available to condition processors (go's `FilterContext`). */
export interface FilterContext {
  /** Retrieves the field/group props at a schema path key inside the
   * metadata model. */
  getFieldGroupByPathKey(pathKey: string): Value;
  /** When `true`, processor errors default the condition to `false`
   * instead of propagating. (Named `shouldSilenceErrors` because the
   * DataFilter class exposes the flag as a `silenceErrors` field.) */
  shouldSilenceErrors(): boolean;
}

/** A condition processor: decides whether `valueFound` satisfies
 * `condition` with the parameters in `filterValue` (go's `ConditionTrue`). */
export type ConditionProcessor = (
  ctx: FilterContext,
  pathKey: string,
  condition: string,
  valueFound: unknown,
  filterValue: Value,
) => boolean;
