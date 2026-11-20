// Data-driven suite for src/value.ts (ADR 0004/0009): Value type alias,
// typeName, guards, get, object builder.

import { test } from "vitest";
import { TestReport, newTestTitle } from "@dyn-rt/test-support";
import { type Value, typeName, isNull, isBoolean, isNumber, isString, isArray, isObject, asArray, asObject, get, object } from "../src/index.js";

interface TypeNameCase {
  testTitle?: string;
  value: Value;
  expected: string;
}

const typeNameCases: TypeNameCase[] = [
  { testTitle: "null", value: null, expected: "null" },
  { testTitle: "boolean", value: true, expected: "boolean" },
  { testTitle: "number", value: 42.5, expected: "number" },
  { testTitle: "string", value: "hello", expected: "string" },
  { testTitle: "array", value: [1, "two", null], expected: "array" },
  { testTitle: "object", value: { a: 1 }, expected: "object" },
];

test("typeName cases (src/value.ts)", () => {
  const report = new TestReport("typeName_cases", "typeName over Value (src/value.ts)");
  for (const testCase of typeNameCases) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const got = typeName(testCase.value);
    if (got !== testCase.expected) {
      report.failCase(`expected ${testCase.expected}, got ${got}`);
    }
  }
  report.finish();
});

interface GuardCase {
  testTitle?: string;
  value: Value;
  // which guard is expected to accept the value
  expectedGuard: "isNull" | "isBoolean" | "isNumber" | "isString" | "isArray" | "isObject";
}

const guardCases: GuardCase[] = [
  { testTitle: "null passes isNull only", value: null, expectedGuard: "isNull" },
  { testTitle: "boolean passes isBoolean only", value: false, expectedGuard: "isBoolean" },
  { testTitle: "number passes isNumber only", value: 7, expectedGuard: "isNumber" },
  { testTitle: "string passes isString only", value: "x", expectedGuard: "isString" },
  { testTitle: "array passes isArray (and not isObject)", value: [], expectedGuard: "isArray" },
  { testTitle: "object passes isObject (and not isArray)", value: {}, expectedGuard: "isObject" },
];

const guards = { isNull, isBoolean, isNumber, isString, isArray, isObject } as const;

test("guard cases (src/value.ts)", () => {
  const report = new TestReport("guard_cases", "type guards over Value (src/value.ts)");
  for (const testCase of guardCases) {
    report.beginCase(newTestTitle(testCase.testTitle));
    for (const [name, guard] of Object.entries(guards)) {
      const accepted = guard(testCase.value);
      if (accepted !== (name === testCase.expectedGuard)) {
        report.failCase(`${name} returned ${accepted} for ${JSON.stringify(testCase.value)}`);
      }
    }
  }
  report.finish();
});

interface GetCase {
  testTitle?: string;
  value: Value;
  key: string;
  expected: Value | undefined;
}

const getCases: GetCase[] = [
  {
    testTitle: "get returns a member",
    value: { a: 1, b: "two" },
    key: "b",
    expected: "two",
  },
  {
    testTitle: "get returns undefined for a missing key",
    value: { a: 1 },
    key: "zzz",
    expected: undefined,
  },
  {
    testTitle: "get returns undefined on a non-object",
    value: [1, 2],
    key: "a",
    expected: undefined,
  },
];

test("get cases (src/value.ts)", () => {
  const report = new TestReport("get_cases", "get member lookup (src/value.ts)");
  for (const testCase of getCases) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const got = get(testCase.value, testCase.key);
    if (JSON.stringify(got) !== JSON.stringify(testCase.expected)) {
      report.failCase(`expected ${JSON.stringify(testCase.expected)}, got ${JSON.stringify(got)}`);
    }
  }
  report.finish();
});

interface ObjectCase {
  testTitle?: string;
  entries: [string, Value][];
  expected: Value;
}

const objectCases: ObjectCase[] = [
  {
    testTitle: "object builds from entries in order",
    entries: [
      ["a", 1],
      ["b", "two"],
    ],
    expected: { a: 1, b: "two" },
  },
  {
    testTitle: "object dedupes keys, last wins",
    entries: [
      ["a", 1],
      ["a", 2],
    ],
    expected: { a: 2 },
  },
];

test("object builder cases (src/value.ts)", () => {
  const report = new TestReport("object_cases", "object() builder incl. dedupe (src/value.ts)");
  for (const testCase of objectCases) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const got = object(testCase.entries);
    if (JSON.stringify(got) !== JSON.stringify(testCase.expected)) {
      report.failCase(`expected ${JSON.stringify(testCase.expected)}, got ${JSON.stringify(got)}`);
    }
    // asArray/asObject round-trip on the built value
    if (asObject(got) === undefined) {
      report.failCase("asObject rejected a built object");
    }
    if (asArray(got) !== undefined) {
      report.failCase("asArray accepted an object");
    }
  }
  report.finish();
});
