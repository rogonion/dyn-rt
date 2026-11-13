// Data-driven suite for src/kind.ts (ADR 0004/0009): kindOf discrimination of
// built-in types (null, undefined, bigint, arrays, Dates, boxed Number) vs
// plain objects vs user-defined class instances, plus isPlainObject /
// isClassInstance / className.

import { test } from "vitest";
import { TestReport, newTestTitle } from "@dyn-rt/test-support";
import {
  type ValueKind,
  kindOf,
  isPlainObject,
  isClassInstance,
  className,
} from "../src/index.js";

class User {
  constructor(
    public ID: number,
    public Name: string,
  ) {}
}

interface KindCase {
  testTitle?: string;
  value: unknown;
  expected: ValueKind;
}

const kindCases: KindCase[] = [
  { testTitle: "null", value: null, expected: "null" },
  { testTitle: "undefined", value: undefined, expected: "undefined" },
  { testTitle: "boolean", value: false, expected: "boolean" },
  { testTitle: "number", value: 3.14, expected: "number" },
  { testTitle: "bigint", value: 10n, expected: "bigint" },
  { testTitle: "string", value: "abc", expected: "string" },
  { testTitle: "symbol", value: Symbol("s"), expected: "symbol" },
  { testTitle: "function", value: () => 1, expected: "function" },
  { testTitle: "array", value: [1, 2], expected: "array" },
  { testTitle: "date", value: new Date("2024-01-02T03:04:05Z"), expected: "date" },
  { testTitle: "plain object", value: { a: 1 }, expected: "plain-object" },
  { testTitle: "null-prototype object is plain", value: Object.create(null), expected: "plain-object" },
  { testTitle: "class instance", value: new User(1, "Alice"), expected: "class-instance" },
  { testTitle: "boxed Number is a class-instance", value: new Number(5), expected: "class-instance" },
  { testTitle: "Map is a class-instance", value: new Map(), expected: "class-instance" },
];

test("kindOf cases (src/kind.ts)", () => {
  const report = new TestReport("kindOf_cases", "kindOf over runtime values (src/kind.ts)");
  for (const testCase of kindCases) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const got = kindOf(testCase.value);
    if (got !== testCase.expected) {
      report.failCase(`expected ${testCase.expected}, got ${got}`);
    }
  }
  report.finish();
});

interface HelperCase {
  testTitle?: string;
  value: unknown;
  expectedPlain: boolean;
  expectedClassInstance: boolean;
  expectedClassName: string | undefined;
}

const helperCases: HelperCase[] = [
  {
    testTitle: "plain object",
    value: { a: 1 },
    expectedPlain: true,
    expectedClassInstance: false,
    expectedClassName: undefined,
  },
  {
    testTitle: "User class instance",
    value: new User(7, "Bob"),
    expectedPlain: false,
    expectedClassInstance: true,
    expectedClassName: "User",
  },
  {
    testTitle: "boxed Number reports className Number",
    value: new Number(5),
    expectedPlain: false,
    expectedClassInstance: true,
    expectedClassName: "Number",
  },
  {
    testTitle: "array is neither plain nor class-instance",
    value: [1],
    expectedPlain: false,
    expectedClassInstance: false,
    expectedClassName: undefined,
  },
];

test("isPlainObject/isClassInstance/className cases (src/kind.ts)", () => {
  const report = new TestReport(
    "kind_helper_cases",
    "isPlainObject / isClassInstance / className (src/kind.ts)",
  );
  for (const testCase of helperCases) {
    report.beginCase(newTestTitle(testCase.testTitle));
    if (isPlainObject(testCase.value) !== testCase.expectedPlain) {
      report.failCase(`isPlainObject expected ${testCase.expectedPlain}`);
    }
    if (isClassInstance(testCase.value) !== testCase.expectedClassInstance) {
      report.failCase(`isClassInstance expected ${testCase.expectedClassInstance}`);
    }
    if (testCase.expectedClassName !== undefined) {
      if (typeof testCase.value !== "object" || testCase.value === null) {
        report.failCase("expected an object for className");
      } else if (className(testCase.value) !== testCase.expectedClassName) {
        report.failCase(
          `className expected ${testCase.expectedClassName}, got ${className(testCase.value)}`,
        );
      }
    }
  }
  report.finish();
});
