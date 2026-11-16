// Filter subdomain application use case (port of
// libs/rust/metadatamodel/src/filter/application.rs, itself ported from
// go-metadatamodel/filter's `DataFilter`): evaluate a query condition
// against every entry of a root sequence in the source data and return the
// indexes that DID NOT pass.
//
// Query condition shapes (go's doc.go example):
// - `{"Type": "LogicalOperator", "LogicalOperator": "And"|"Or", "Negate": bool,
//     "Value": [<query condition>, ...]}`
// - `{"Type": "FieldGroup", "Negate": bool, "Value": {"<schema path key>": {"<condition>": {...}}}}`

import { DynObject } from "@dyn-rt/manipulation";
import { asArray, asObject, get, isObject, type Value } from "@dyn-rt/shared";
import { dataPathKeepWildcards, parseTemplate } from "../core/index.js";
import type { MetadataModel } from "../core/domain.js";
import { MetadatamodelError } from "../error.js";
import { defaultFilterProcessors } from "./conditions.js";
import {
  query,
  parseLogicalOperator,
  type ConditionProcessor,
  type FilterContext,
  type LogicalOperator,
} from "./domain.js";

/** Filters source data whose structure is defined by a metadata model (go's
 * `DataFilter`). */
export class DataFilter implements FilterContext {
  /** Condition processors keyed by condition name. */
  processors: [string, ConditionProcessor][] = defaultFilterProcessors();
  /** When `true`, errors default the current condition to `false`. */
  silenceErrors = false;
  /** Sub-set of the metadata model acting as root context (default `$`). */
  rootJsonPathKey = "$";
  /** Path to the data in `source` acting as root context (default `$`). */
  rootJsonPathToValue = "$";

  constructor(
    /** The source data to filter (the root value at `rootJsonPathToValue`
     * must be a sequence), as a DynObject handle. */
    public readonly source: DynObject,
    /** The metadata model describing the source data. */
    public readonly metadataModel: MetadataModel,
  ) {}

  /** Runs `queryCondition` against every entry of the root sequence and
   * returns the indexes that did NOT pass the filter (go's
   * `DataFilter.Filter`). Empty `rootJsonPathKey`/`rootJsonPathToValue`
   * keep the current values. */
  filter(queryCondition: Value, rootJsonPathKey = "", rootJsonPathToValue = ""): number[] {
    if (rootJsonPathKey !== "") {
      this.rootJsonPathKey = rootJsonPathKey;
    }
    if (rootJsonPathToValue !== "") {
      this.rootJsonPathToValue = rootJsonPathToValue;
    }

    const rootSegments = parseTemplate(this.rootJsonPathToValue);
    let rootValues: unknown[];
    try {
      rootValues = this.source.get(rootSegments);
    } catch (err) {
      throw MetadatamodelError.manipulation(err instanceof Error ? err.message : String(err));
    }
    const rootValue = rootValues[0];
    if (rootValue === undefined) {
      throw MetadatamodelError.filter("get root value yielded 0 results");
    }
    if (!Array.isArray(rootValue)) {
      throw MetadatamodelError.filter("root value should be a sequence");
    }

    // iterate the root sequence's entries: `<root>[*]`
    const elementsPath = `${this.rootJsonPathToValue}[*]`;
    const elementsSegments = parseTemplate(elementsPath);
    let elements: unknown[];
    try {
      elements = this.source.get(elementsSegments);
    } catch (err) {
      throw MetadatamodelError.manipulation(err instanceof Error ? err.message : String(err));
    }

    const excludeIndexes: number[] = [];
    const queryIsEmpty = !isObject(queryCondition) || Object.keys(queryCondition).length === 0;
    for (let index = 0; index < elements.length; index++) {
      if (queryIsEmpty) {
        continue; // empty query keeps everything
      }
      const ok = this.isQueryConditionTrue(queryCondition, elements[index]);
      if (!ok) {
        excludeIndexes.push(index);
      }
    }
    return excludeIndexes;
  }

  /** Evaluates one query condition against the current value (go's
   * `isQueryConditionTrue`). */
  private isQueryConditionTrue(queryCondition: Value, currentValue: unknown): boolean {
    const typeRaw = get(queryCondition, query.TYPE);
    if (typeof typeRaw !== "string") {
      return this.errorOrFalse(`key '${query.TYPE}' is not valid`);
    }
    if (typeRaw === query.TYPE_LOGICAL_OPERATOR) {
      return this.isLogicalOperatorTrue(queryCondition, currentValue);
    }
    if (typeRaw === query.TYPE_FIELD_GROUP) {
      return this.isFieldGroupTrue(queryCondition, currentValue);
    }
    return this.errorOrFalse(`unknown query condition type: ${typeRaw}`);
  }

  /** `LogicalOperator` condition: `Value` is an array of sub-conditions
   * combined with And/Or, then flipped when `Negate` (go's
   * `isRecursiveLogicalOperatorTrue`). */
  private isLogicalOperatorTrue(queryCondition: Value, currentValue: unknown): boolean {
    const negate = negateOf(queryCondition);
    const operator = logicalOperatorOf(this, queryCondition);
    const valueRaw = get(queryCondition, query.VALUE);
    const conditions = valueRaw !== undefined ? asArray(valueRaw) : undefined;
    if (conditions === undefined) {
      return this.errorOrFalse(`key '${query.VALUE}' is not valid`);
    }

    const results: boolean[] = [];
    for (const condition of conditions) {
      if (!isObject(condition)) {
        return this.errorOrFalse("condition is not an object");
      }
      results.push(this.isQueryConditionTrue(condition, currentValue));
    }

    const combined =
      operator === "Or" ? results.some((r) => r) : results.every((r) => r);
    return negate ? !combined : combined;
  }

  /** `FieldGroup` condition: `Value` is an object mapping schema path keys
   * to per-field conditions, combined with And/Or, then flipped when
   * `Negate` (go's `isRecursiveFieldGroupTrue`). */
  private isFieldGroupTrue(queryCondition: Value, currentValue: unknown): boolean {
    const negate = negateOf(queryCondition);
    const operator = logicalOperatorOf(this, queryCondition);
    const valueRaw = get(queryCondition, query.VALUE);
    const conditions = valueRaw !== undefined ? asObject(valueRaw) : undefined;
    if (conditions === undefined) {
      return this.errorOrFalse(`key '${query.VALUE}' is not valid`);
    }

    const results: boolean[] = [];
    for (const [pathKey, condition] of Object.entries(conditions)) {
      if (!isObject(condition)) {
        return this.errorOrFalse("condition is not an object");
      }
      results.push(this.isFieldGroupConditionTrue(pathKey, condition, currentValue));
    }

    const combined =
      operator === "Or" ? results.some((r) => r) : results.every((r) => r);
    return negate ? !combined : combined;
  }

  /** One field's conditions: resolves the schema path key to a data path
   * (wildcards kept), iterates every matched value, and ANDs the
   * conditions; OR over the matched values (go's
   * `isFieldGroupConditionTrue`). */
  private isFieldGroupConditionTrue(
    pathKey: string,
    queryCondition: Value,
    currentValue: unknown,
  ): boolean {
    const conditions = asObject(queryCondition);
    if (conditions === undefined || Object.keys(conditions).length === 0) {
      return this.errorOrFalse("query condition is empty");
    }

    // root-relative schema path key -> path to data in the source value
    const currentKey = pathKey.replace(this.rootJsonPathKey, "$");
    const toValue = dataPathKeepWildcards(currentKey);
    const segments = parseTemplate(toValue);

    let orConditionTrue = false;
    let loopError: MetadatamodelError | undefined;
    try {
      DynObject.new(currentValue).forEach(segments, (value) => {
        // AND over this field's conditions
        for (const [conditionKey, conditionData] of Object.entries(conditions)) {
          const processor = this.processor(conditionKey);
          if (processor === undefined) {
            if (this.silenceErrors) {
              continue;
            }
            loopError = MetadatamodelError.filter(
              `no processor for filter condition '${conditionKey}'`,
            );
            return false;
          }
          let conditionTrue: boolean;
          try {
            conditionTrue = processor(this, pathKey, conditionKey, value, conditionData);
          } catch (err) {
            if (this.silenceErrors) {
              continue;
            }
            loopError =
              err instanceof MetadatamodelError
                ? err
                : MetadatamodelError.filter(err instanceof Error ? err.message : String(err));
            return false;
          }
          if (!conditionTrue) {
            return true; // this value fails the AND: try the next value
          }
        }
        // all conditions true for this value
        orConditionTrue = true;
        return false; // stop iterating
      });
    } catch (err) {
      return this.errorOrFalse(err instanceof Error ? err.message : String(err));
    }
    if (loopError !== undefined) {
      return this.errorOrFalseErr(loopError);
    }
    return orConditionTrue;
  }

  private processor(conditionKey: string): ConditionProcessor | undefined {
    return this.processors.find(([name]) => name === conditionKey)?.[1];
  }

  /** When `silenceErrors` is set, errors become `false` (go's
   * `returnErrorOrFalse`). */
  private errorOrFalse(message: string): boolean {
    if (this.silenceErrors) {
      return false;
    }
    throw MetadatamodelError.filter(message);
  }

  private errorOrFalseErr(err: MetadatamodelError): boolean {
    if (this.silenceErrors) {
      return false;
    }
    throw err;
  }

  // FilterContext -----------------------------------------------------------

  getFieldGroupByPathKey(pathKey: string): Value {
    const segments = parseTemplate(pathKey);
    let values: unknown[];
    try {
      values = DynObject.new(this.metadataModel).get(segments);
    } catch (err) {
      throw MetadatamodelError.manipulation(err instanceof Error ? err.message : String(err));
    }
    const first = values[0];
    if (first === undefined) {
      throw MetadatamodelError.filter("get field/group failed");
    }
    // the metadata model is a Value tree, so anything found is a Value
    return first as Value;
  }

  shouldSilenceErrors(): boolean {
    return this.silenceErrors;
  }
}

/** The `Negate` flag of a query condition (default false). */
function negateOf(queryCondition: Value): boolean {
  return get(queryCondition, query.NEGATE) === true;
}

/** The `LogicalOperator` of a query condition (default And; must be valid). */
function logicalOperatorOf(ctx: DataFilter, queryCondition: Value): LogicalOperator {
  const rawRaw = get(queryCondition, query.LOGICAL_OPERATOR);
  const raw = typeof rawRaw === "string" ? rawRaw : query.AND;
  const parsed = parseLogicalOperator(raw);
  if (parsed !== undefined) {
    return parsed;
  }
  if (ctx.silenceErrors) {
    return "And";
  }
  throw MetadatamodelError.filter(`invalid logical operator '${raw}'`);
}
