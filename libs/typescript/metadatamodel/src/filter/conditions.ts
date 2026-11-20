// Condition processors (port of
// libs/rust/metadatamodel/src/filter/conditions.rs; go's
// `IsConditionTrue` / `IsNumberOfEntriesConditionTrue` /
// `IsTextConditionTrue` / `IsNumberConditionTrue` / `IsTimestampConditionTrue`
// / `IsDefaultEqualTrue`): decide whether a value found in the source data
// satisfies one filter condition.

import { areEqual } from "@dyn-rt/manipulation";
import { asArray, get, type Value } from "@dyn-rt/shared";
import { MetadatamodelError } from "../error.js";
import { condition, conditionProps, datetimeFormat, fieldType, type ConditionProcessor, type FilterContext } from "./domain.js";

/** The default set of condition processors, keyed by condition name (go's
 * `DefaultFilterProcessors`). */
export function defaultFilterProcessors(): [string, ConditionProcessor][] {
  return [
    [condition.NO_OF_ENTRIES_GREATER_THAN, isConditionTrue],
    [condition.NO_OF_ENTRIES_LESS_THAN, isConditionTrue],
    [condition.NO_OF_ENTRIES_EQUAL_TO, isConditionTrue],
    [condition.GREATER_THAN, isConditionTrue],
    [condition.LESS_THAN, isConditionTrue],
    [condition.EQUAL_TO, isConditionTrue],
    [condition.BEGINS_WITH, isConditionTrue],
    [condition.ENDS_WITH, isConditionTrue],
    [condition.CONTAINS, isConditionTrue],
  ];
}

/** Dispatches one filter condition (go's `IsConditionTrue`): entry-count
 * conditions go to `numberOfEntriesConditionTrue`; everything else
 * is dispatched on the condition's `AssumedFieldType` — and when the value
 * found is a sequence, ANY element satisfying the condition counts. */
export function isConditionTrue(ctx: FilterContext, pathKey: string, filterCondition: string, valueFound: unknown, filterValue: Value): boolean {
  if (filterCondition === condition.NO_OF_ENTRIES_GREATER_THAN || filterCondition === condition.NO_OF_ENTRIES_LESS_THAN || filterCondition === condition.NO_OF_ENTRIES_EQUAL_TO) {
    return numberOfEntriesConditionTrue(ctx, pathKey, filterCondition, valueFound, filterValue);
  }

  const assumedRaw = get(filterValue, conditionProps.ASSUMED_FIELD_TYPE);
  if (typeof assumedRaw !== "string") {
    if (assumedRaw === undefined) {
      return errorOrFalse(ctx, `filter condition property '${conditionProps.ASSUMED_FIELD_TYPE}' not found`);
    }
    return errorOrFalse(ctx, `filter condition property '${conditionProps.ASSUMED_FIELD_TYPE}' is not a string`);
  }
  const assumed = assumedRaw;

  if (Array.isArray(valueFound)) {
    for (const element of valueFound) {
      const conditionTrue = dispatchAssumedType(ctx, pathKey, filterCondition, element, filterValue, assumed);
      if (conditionTrue) {
        return true;
      }
    }
    return false;
  }

  return dispatchAssumedType(ctx, pathKey, filterCondition, valueFound, filterValue, assumed);
}

/** Routes to the text/number/timestamp/default processor by assumed field
 * type (go's switch in `IsConditionTrue`). */
function dispatchAssumedType(ctx: FilterContext, pathKey: string, filterCondition: string, valueFound: unknown, filterValue: Value, assumed: string): boolean {
  if (assumed === fieldType.TEXT) {
    return textConditionTrue(ctx, pathKey, filterCondition, valueFound, filterValue);
  }
  if (assumed === fieldType.NUMBER) {
    return numberConditionTrue(ctx, pathKey, filterCondition, valueFound, filterValue);
  }
  if (assumed === fieldType.TIMESTAMP) {
    return timestampConditionTrue(ctx, pathKey, filterCondition, valueFound, filterValue);
  }
  return defaultEqualTrue(ctx, pathKey, filterCondition, valueFound, filterValue);
}

/** `NoOfEntries*` conditions: compares the length of a sequence value
 * (go's `IsNumberOfEntriesConditionTrue`). */
export function numberOfEntriesConditionTrue(ctx: FilterContext, _pathKey: string, filterCondition: string, valueFound: unknown, filterValue: Value): boolean {
  if (!Array.isArray(valueFound)) {
    return false;
  }
  const length = valueFound.length;

  const values = compareValuesI64(ctx, filterValue);
  for (const value of values) {
    let hit: boolean;
    if (filterCondition === condition.NO_OF_ENTRIES_EQUAL_TO) {
      hit = value === length;
    } else if (filterCondition === condition.NO_OF_ENTRIES_GREATER_THAN) {
      hit = value > length;
    } else if (filterCondition === condition.NO_OF_ENTRIES_LESS_THAN) {
      hit = value < length;
    } else {
      return errorOrFalse(ctx, `unsupported filter condition '${filterCondition}'`);
    }
    if (hit) {
      return true;
    }
  }
  return false;
}

/** Text conditions: EqualTo / BeginsWith / EndsWith / Contains against
 * string values, honoring `CaseInsensitive` (go's `IsTextConditionTrue`). */
export function textConditionTrue(ctx: FilterContext, _pathKey: string, filterCondition: string, valueFound: unknown, filterValue: Value): boolean {
  const caseInsensitiveRaw = get(filterValue, conditionProps.CASE_INSENSITIVE);
  const caseInsensitive = caseInsensitiveRaw === true;

  const valuesToCompare: string[] = [];
  const single = get(filterValue, conditionProps.VALUE);
  const multiple = get(filterValue, conditionProps.VALUES);
  if (single !== undefined) {
    if (typeof single !== "string") {
      return errorOrFalse(ctx, `filter condition property '${conditionProps.VALUE}' is not a string`);
    }
    valuesToCompare.push(caseInsensitive ? single.toLowerCase() : single);
  } else if (multiple !== undefined) {
    const items = asArray(multiple);
    if (items === undefined) {
      return errorOrFalse(ctx, `filter condition property '${conditionProps.VALUES}' is not an array`);
    }
    for (const item of items) {
      if (typeof item !== "string") {
        return errorOrFalse(ctx, `filter condition property '${conditionProps.VALUES}' entries must be strings`);
      }
      valuesToCompare.push(caseInsensitive ? item.toLowerCase() : item);
    }
  } else {
    return errorOrFalse(ctx, `filter condition property '${conditionProps.VALUE}' not found`);
  }

  if (typeof valueFound !== "string") {
    return false;
  }
  const haystack = caseInsensitive ? valueFound.toLowerCase() : valueFound;

  for (const value of valuesToCompare) {
    let hit: boolean;
    if (filterCondition === condition.EQUAL_TO) {
      hit = haystack === value;
    } else if (filterCondition === condition.BEGINS_WITH) {
      hit = haystack.startsWith(value);
    } else if (filterCondition === condition.ENDS_WITH) {
      hit = haystack.endsWith(value);
    } else if (filterCondition === condition.CONTAINS) {
      hit = haystack.includes(value);
    } else {
      return errorOrFalse(ctx, `unsupported filter condition '${filterCondition}'`);
    }
    if (hit) {
      return true;
    }
  }
  return false;
}

/** Number conditions: EqualTo / GreaterThan / LessThan against numeric
 * values (go's `IsNumberConditionTrue`). */
export function numberConditionTrue(ctx: FilterContext, _pathKey: string, filterCondition: string, valueFound: unknown, filterValue: Value): boolean {
  const values = compareValuesF64(ctx, filterValue);

  const valueFoundFloat = asF64(valueFound);
  if (valueFoundFloat === undefined) {
    return false;
  }

  for (const value of values) {
    let hit: boolean;
    if (filterCondition === condition.EQUAL_TO) {
      hit = valueFoundFloat === value;
    } else if (filterCondition === condition.GREATER_THAN) {
      hit = valueFoundFloat > value;
    } else if (filterCondition === condition.LESS_THAN) {
      hit = valueFoundFloat < value;
    } else {
      return errorOrFalse(ctx, `unsupported filter condition '${filterCondition}'`);
    }
    if (hit) {
      return true;
    }
  }
  return false;
}

/** Timestamp conditions: EqualTo / GreaterThan / LessThan on RFC 3339
 * timestamps, compared at the granularity of the condition's
 * `DateTimeFormat` (go's `IsTimestampConditionTrue`). */
export function timestampConditionTrue(ctx: FilterContext, _pathKey: string, filterCondition: string, valueFound: unknown, filterValue: Value): boolean {
  const formatRaw = get(filterValue, conditionProps.DATE_TIME_FORMAT);
  if (typeof formatRaw !== "string") {
    if (formatRaw === undefined) {
      return errorOrFalse(ctx, `filter condition property '${conditionProps.DATE_TIME_FORMAT}' not found`);
    }
    return errorOrFalse(ctx, `filter condition property '${conditionProps.DATE_TIME_FORMAT}' is not a string`);
  }
  const format = formatRaw;

  // collect the comparison values as parsed timestamps
  const rawValues: Value[] = [];
  const single = get(filterValue, conditionProps.VALUE);
  const multiple = get(filterValue, conditionProps.VALUES);
  if (single !== undefined) {
    rawValues.push(single);
  } else if (multiple !== undefined) {
    const items = asArray(multiple);
    if (items === undefined) {
      return errorOrFalse(ctx, `filter condition property '${conditionProps.VALUES}' is not an array`);
    }
    rawValues.push(...items);
  } else {
    return errorOrFalse(ctx, `filter condition property '${conditionProps.VALUE}' not found`);
  }
  const valuesToCompare: Timestamp[] = [];
  for (const raw of rawValues) {
    if (typeof raw !== "string") {
      return errorOrFalse(ctx, `filter condition property '${conditionProps.VALUE}' must be an RFC 3339 string`);
    }
    const ts = Timestamp.parseRfc3339(raw);
    if (ts === undefined) {
      return errorOrFalse(ctx, `error parsing filter condition value \`${raw}\` to time`);
    }
    valuesToCompare.push(ts);
  }

  if (typeof valueFound !== "string") {
    return false;
  }
  const valueFoundTime = Timestamp.parseRfc3339(valueFound);
  if (valueFoundTime === undefined) {
    return false;
  }

  for (const value of valuesToCompare) {
    const ordering = compareParts(valueFoundTime.partsForFormat(format), value.partsForFormat(format));
    let hit: boolean;
    if (filterCondition === condition.EQUAL_TO) {
      hit = ordering === 0;
    } else if (filterCondition === condition.GREATER_THAN) {
      hit = ordering > 0;
    } else if (filterCondition === condition.LESS_THAN) {
      hit = ordering < 0;
    } else {
      return errorOrFalse(ctx, `unsupported filter condition '${filterCondition}'`);
    }
    if (hit) {
      return true;
    }
  }
  return false;
}

/** Default (Any-typed) condition: EqualTo via deep equality (go's
 * `IsDefaultEqualTrue`). */
export function defaultEqualTrue(ctx: FilterContext, _pathKey: string, filterCondition: string, valueFound: unknown, filterValue: Value): boolean {
  const rawValues: Value[] = [];
  const single = get(filterValue, conditionProps.VALUE);
  const multiple = get(filterValue, conditionProps.VALUES);
  if (single !== undefined) {
    rawValues.push(single);
  } else if (multiple !== undefined) {
    const items = asArray(multiple);
    if (items === undefined) {
      return errorOrFalse(ctx, `filter condition property '${conditionProps.VALUES}' is not an array`);
    }
    rawValues.push(...items);
  } else {
    return errorOrFalse(ctx, `filter condition property '${conditionProps.VALUE}' or '${conditionProps.VALUES}' not found`);
  }

  if (filterCondition === condition.EQUAL_TO) {
    for (const value of rawValues) {
      if (areEqual(valueFound, value)) {
        return true;
      }
    }
    return false;
  }
  return errorOrFalse(ctx, `unsupported filter condition '${filterCondition}'`);
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/** `Value`/`Values` of the filter condition as integers (go converts to int). */
function compareValuesI64(ctx: FilterContext, filterValue: Value): number[] {
  const raw = rawCompareValues(ctx, filterValue);
  const out: number[] = [];
  for (const value of raw) {
    if (typeof value !== "number") {
      return errorOrEmpty(ctx, "filter condition value is not a number");
    }
    out.push(Math.trunc(value));
  }
  return out;
}

/** `Value`/`Values` of the filter condition as floats (numeric strings are
 * parsed too, mirroring go's conversion to float64). */
function compareValuesF64(ctx: FilterContext, filterValue: Value): number[] {
  const raw = rawCompareValues(ctx, filterValue);
  const out: number[] = [];
  for (const value of raw) {
    const f = asF64Value(value);
    if (f === undefined) {
      return errorOrEmpty(ctx, "filter condition value is not a number");
    }
    out.push(f);
  }
  return out;
}

/** The raw `Value`/`Values` entries of a filter condition. */
function rawCompareValues(ctx: FilterContext, filterValue: Value): Value[] {
  const single = get(filterValue, conditionProps.VALUE);
  if (single !== undefined) {
    return [single];
  }
  const multiple = get(filterValue, conditionProps.VALUES);
  if (multiple !== undefined) {
    const items = asArray(multiple);
    if (items === undefined) {
      return errorOrEmpty(ctx, `filter condition property '${conditionProps.VALUES}' is not an array`);
    }
    return items;
  }
  return errorOrEmpty(ctx, `filter condition property '${conditionProps.VALUE}' not found`);
}

/** Numeric view of a runtime value (numbers as-is, numeric strings parsed). */
function asF64(value: unknown): number | undefined {
  if (typeof value === "number") return value;
  if (typeof value === "bigint") return Number(value);
  if (typeof value === "string") return asF64Value(value);
  return undefined;
}

/** Numeric view of a `Value` (numbers as-is, numeric strings parsed). */
function asF64Value(value: Value): number | undefined {
  if (typeof value === "number") return value;
  if (typeof value === "string") {
    const f = Number(value.trim());
    return Number.isNaN(f) ? undefined : f;
  }
  return undefined;
}

/** When the context silences errors, the result defaults (`false` / empty
 * array) instead of failing (go's `returnErrorOrFalse`). */
function errorOrFalse(ctx: FilterContext, message: string): boolean {
  if (ctx.shouldSilenceErrors()) {
    return false;
  }
  throw MetadatamodelError.filter(message);
}

function errorOrEmpty<T>(ctx: FilterContext, message: string): T[] {
  if (ctx.shouldSilenceErrors()) {
    return [];
  }
  throw MetadatamodelError.filter(message);
}

// ---------------------------------------------------------------------------
// timestamps (explicit RFC 3339 parsing — no date library in core per
// ADR 0002; port of the Rust filter/conditions.rs Timestamp)
// ---------------------------------------------------------------------------

/** A parsed RFC 3339 timestamp: civil date-time parts plus the UTC offset
 * in minutes. */
export class Timestamp {
  constructor(
    public readonly year: number,
    public readonly month: number,
    public readonly day: number,
    public readonly hour: number,
    public readonly minute: number,
    public readonly second: number,
    public readonly offsetMinutes: number,
  ) {}

  /** Parses `YYYY-MM-DDTHH:MM:SS[.fraction][Z|±HH:MM]` explicitly. */
  static parseRfc3339(input: string): Timestamp | undefined {
    if (input.length < 20) {
      return undefined;
    }
    // YYYY-MM-DD
    const year = parseDigits(input.slice(0, 4));
    if (year === undefined || input[4] !== "-") return undefined;
    const month = parseDigits(input.slice(5, 7));
    if (month === undefined || input[7] !== "-") return undefined;
    const day = parseDigits(input.slice(8, 10));
    if (day === undefined) return undefined;
    const dateTimeSep = input[10];
    if (dateTimeSep !== "T" && dateTimeSep !== " ") return undefined;
    const hour = parseDigits(input.slice(11, 13));
    if (hour === undefined || input[13] !== ":") return undefined;
    const minute = parseDigits(input.slice(14, 16));
    if (minute === undefined || input[16] !== ":") return undefined;
    const second = parseDigits(input.slice(17, 19));
    if (second === undefined) return undefined;

    // optional fractional seconds (skipped — second granularity is
    // enough for the supported datetime formats)
    let rest = input.slice(19);
    if (rest.startsWith(".")) {
      let end = 0;
      while (end < rest.length && isAsciiDigit(rest.charCodeAt(end))) end++;
      rest = rest.slice(end);
    }

    // zone: Z or ±HH:MM
    let offsetMinutes: number;
    if (rest === "Z" || rest === "") {
      offsetMinutes = 0;
    } else if (rest.startsWith("+") || rest.startsWith("-")) {
      const sign = rest.startsWith("-") ? -1 : 1;
      const offset = rest.slice(1);
      if (offset.length < 5 || offset[2] !== ":") return undefined;
      const oh = parseDigits(offset.slice(0, 2));
      const om = parseDigits(offset.slice(3, 5));
      if (oh === undefined || om === undefined) return undefined;
      offsetMinutes = sign * (oh * 60 + om);
    } else {
      return undefined;
    }

    return new Timestamp(year, month, day, hour, minute, second, offsetMinutes);
  }

  /** The comparison parts at the granularity of the datetime format
   * (go compares year, then month, then day, ... per format). */
  partsForFormat(format: string): number[] {
    if (format === datetimeFormat.YYYY) return [this.year];
    if (format === datetimeFormat.YYYYMM) return [this.year, this.month];
    if (format === datetimeFormat.YYYYMMDD) return [this.year, this.month, this.day];
    if (format === datetimeFormat.YYYYMMDDHHMM) {
      return [this.year, this.month, this.day, this.hour, this.minute];
    }
    if (format === datetimeFormat.MM) return [this.month];
    if (format === datetimeFormat.HHMM) return [this.hour, this.minute];
    // unknown format: full timestamp
    return [this.year, this.month, this.day, this.hour, this.minute, this.second];
  }
}

/** Lexicographic comparison of two part vectors. */
function compareParts(a: number[], b: number[]): number {
  const len = Math.max(a.length, b.length);
  for (let i = 0; i < len; i++) {
    const av = a[i] ?? 0;
    const bv = b[i] ?? 0;
    if (av < bv) return -1;
    if (av > bv) return 1;
  }
  return 0;
}

/** Parses exactly the digits of `s` as a number. */
function parseDigits(s: string): number | undefined {
  if (s.length === 0) return undefined;
  for (let i = 0; i < s.length; i++) {
    if (!isAsciiDigit(s.charCodeAt(i))) return undefined;
  }
  return Number(s);
}

function isAsciiDigit(code: number): boolean {
  return code >= 48 && code <= 57;
}
