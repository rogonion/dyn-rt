// Application suite for src/application/are_equal.ts: recursive structural
// equality over values, incl. custom per-constructor checkers (port of
// libs/rust/manipulation/tests/application/are_equal.rs).

import { test } from "vitest";
import { TestReport, newTestTitle } from "@dyn-rt/test-support";
import { areEqual, type CustomEqualMap } from "../src/index.js";
import { UserProfile, Address } from "./common.js";

interface AreEqualCase {
  testTitle?: string;
  left: unknown;
  right: unknown;
  expected: boolean;
  custom?: CustomEqualMap;
}

// custom checker: two UserProfiles are "equal" when only the name matches
function nameOnly(left: unknown, right: unknown): boolean {
  return (
    left instanceof UserProfile && right instanceof UserProfile && left.Name === right.Name
  );
}

function yieldTestData(): AreEqualCase[] {
  const custom: CustomEqualMap = new Map([[UserProfile, nameOnly]]);
  return [
    { testTitle: "equal scalars", left: 1, right: 1, expected: true },
    { testTitle: "unequal scalars", left: 1, right: 2, expected: false },
    {
      // number vs bigint of same numeric value are different kinds
      testTitle: "number vs bigint of same value are different kinds",
      left: 1,
      right: 1n,
      expected: false,
    },
    { testTitle: "null vs undefined are different kinds", left: null, right: undefined, expected: false },
    {
      testTitle: "equal sequences",
      left: [1, "x"],
      right: [1, "x"],
      expected: true,
    },
    {
      testTitle: "sequences of different length",
      left: [1],
      right: [1, 2],
      expected: false,
    },
    {
      // map equality is key-matched, so insertion order does not matter
      testTitle: "maps equal regardless of entry order",
      left: { a: 1, b: 2 },
      right: { b: 2, a: 1 },
      expected: true,
    },
    { testTitle: "maps with different values", left: { a: 1 }, right: { a: 2 }, expected: false },
    { testTitle: "maps with different keys", left: { a: 1 }, right: { b: 1 }, expected: false },
    {
      testTitle: "equal class instances",
      left: new UserProfile({ Name: "Ann", Age: 30 }),
      right: new UserProfile({ Name: "Ann", Age: 30 }),
      expected: true,
    },
    {
      testTitle: "class instances differing in one field",
      left: new UserProfile({ Name: "Ann", Age: 30 }),
      right: new UserProfile({ Name: "Ann", Age: 31 }),
      expected: false,
    },
    {
      testTitle: "struct comparison is by fields, not by class",
      left: new Address({ Street: "s", City: "c", ZipCode: null }),
      right: { Street: "s", City: "c", ZipCode: null },
      expected: false, // class-instance vs plain-object are different kinds
    },
    {
      testTitle: "custom per-constructor checker overrides the default comparison",
      left: new UserProfile({ Name: "Ann", Age: 30 }),
      right: new UserProfile({ Name: "Ann", Age: 99 }),
      expected: true,
      custom,
    },
    {
      testTitle: "Some payloads equal",
      left: { variant: "Some", payload: 1 },
      right: { variant: "Some", payload: 1 },
      expected: true,
    },
    {
      testTitle: "Some vs None-variant",
      left: { variant: "Some", payload: 1 },
      right: { variant: "None" },
      expected: false,
    },
    {
      testTitle: "None vs None",
      left: { variant: "None" },
      right: { variant: "None" },
      expected: true,
    },
    {
      testTitle: "Some payload unwraps and compares against the bare value",
      left: { variant: "Some", payload: 7 },
      right: 7,
      expected: true,
    },
    {
      testTitle: "enum variants with equal payloads",
      left: { variant: "Circle", payload: { Radius: 2.5 } },
      right: { variant: "Circle", payload: { Radius: 2.5 } },
      expected: true,
    },
    {
      testTitle: "different enum variants are unequal even with equal-ish payloads",
      left: { variant: "Circle", payload: { Radius: 2.5 } },
      right: { variant: "Square", payload: { Side: 2.5 } },
      expected: false,
    },
    {
      testTitle: "dates compare by time",
      left: new Date(1000),
      right: new Date(1000),
      expected: true,
    },
    {
      testTitle: "dates with different times are unequal",
      left: new Date(1000),
      right: new Date(2000),
      expected: false,
    },
    {
      testTitle: "arrays compare element-wise",
      left: [1, 2, 3],
      right: [1, 2, 3],
      expected: true,
    },
    {
      testTitle: "nested mixed structures compare deeply",
      left: { users: [new UserProfile({ Name: "a" })], count: 1 },
      right: { count: 1, users: [new UserProfile({ Name: "a" })] },
      expected: true,
    },
  ];
}

test("are_equal_cases", () => {
  const report = new TestReport(
    "are_equal_cases",
    "recursive structural equality over values (src/application/are_equal.ts)",
  );
  for (const testCase of yieldTestData()) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const got = areEqual(testCase.left, testCase.right, testCase.custom);
    if (got !== testCase.expected) {
      report.failCase(`expected ${testCase.expected}, got ${got}`);
    }
  }
  report.finish();
});
