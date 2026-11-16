// Domain-layer suite for reflect (ADR 0004/0009): reflectRef routing and
// reflectTypeName (src/domain.ts).

import { test } from "vitest";
import { TestReport, newTestTitle } from "@dyn-rt/test-support";
import { reflectRef, reflectTypeName, type ReflectRef } from "../src/index.js";

class User {
  constructor(
    public ID: number,
    public Name: string,
  ) {}
}

interface RefCase {
  testTitle?: string;
  value: unknown;
  expected: ReflectRef;
  expectedTypeName: string;
}

const refCases: RefCase[] = [
  { testTitle: "null is Scalar", value: null, expected: "Scalar", expectedTypeName: "null" },
  {
    testTitle: "undefined is Scalar",
    value: undefined,
    expected: "Scalar",
    expectedTypeName: "undefined",
  },
  { testTitle: "number is Scalar", value: 42, expected: "Scalar", expectedTypeName: "number" },
  { testTitle: "bigint is Scalar", value: 10n, expected: "Scalar", expectedTypeName: "bigint" },
  { testTitle: "string is Scalar", value: "x", expected: "Scalar", expectedTypeName: "string" },
  { testTitle: "boolean is Scalar", value: true, expected: "Scalar", expectedTypeName: "boolean" },
  { testTitle: "array is Sequence", value: [1, 2], expected: "Sequence", expectedTypeName: "array" },
  {
    testTitle: "plain object is Map",
    value: { a: 1 },
    expected: "Map",
    expectedTypeName: "plain-object",
  },
  {
    testTitle: "null-prototype object is Map",
    value: Object.create(null),
    expected: "Map",
    expectedTypeName: "plain-object",
  },
  {
    testTitle: "class instance is Struct",
    value: new User(1, "Alice"),
    expected: "Struct",
    expectedTypeName: "User",
  },
  {
    testTitle: "Date is Unknown (opaque leaf)",
    value: new Date(0),
    expected: "Unknown",
    expectedTypeName: "date",
  },
  {
    testTitle: "boxed Number is a Scalar leaf",
    value: new Number(5),
    expected: "Scalar",
    expectedTypeName: "Number",
  },
  {
    testTitle: "function is Unknown",
    value: () => 1,
    expected: "Unknown",
    expectedTypeName: "function",
  },
];

test("reflectRef routing cases (src/domain.ts)", () => {
  const report = new TestReport(
    "reflectRef_cases",
    "reflectRef routing over native values (src/domain.ts)",
  );
  for (const testCase of refCases) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const got = reflectRef(testCase.value);
    if (got !== testCase.expected) {
      report.failCase(`expected ${testCase.expected}, got ${got}`);
    }
    const gotName = reflectTypeName(testCase.value);
    if (gotName !== testCase.expectedTypeName) {
      report.failCase(`typeName: expected ${testCase.expectedTypeName}, got ${gotName}`);
    }
  }
  report.finish();
});
