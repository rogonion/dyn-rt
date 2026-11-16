// Data-driven suite for src/application/set.ts (ADR 0004/0009): cases ported
// from libs/rust/manipulation/tests/application/set.rs (itself ported from
// go-json's object/set_test.go) — including schema-driven coercion and
// creation of intermediate structures.

import { test } from "vitest";
import { TestReport, newTestTitle } from "@dyn-rt/test-support";
import { RegexPathTokenizer, parsePath } from "@dyn-rt/path";
import { reflectClone } from "@dyn-rt/reflect";
import type { Schema } from "@dyn-rt/schema";
import { areEqual, set } from "../src/index.js";
import {
  ComplexData,
  User,
  UserProfile,
  UserWithAddress,
  Address,
  eq,
  render,
  stringIntMapSchema,
  userProfileSchema,
} from "./common.js";

interface SetCase {
  testTitle?: string;
  root: unknown;
  path: string;
  value: unknown;
  schema?: Schema;
  expectedCount: number;
  /** expected root after the set, compared with areEqual (deep, like go) */
  expectedRoot: unknown;
  expectError?: string;
}

function yieldTestData(): SetCase[] {
  return [
    {
      // without a schema the value is assigned as-is (no static target type)
      testTitle: "set existing map entry stores the value as-is without a schema (go Set map entry)",
      root: { a: 1 },
      path: "$.a",
      value: "42",
      expectedCount: 1,
      expectedRoot: { a: "42" },
    },
    {
      testTitle: "set creates a missing map entry",
      root: { a: 1 },
      path: "$.b",
      value: "new",
      expectedCount: 1,
      expectedRoot: { a: 1, b: "new" },
    },
    {
      testTitle: "schema coerces the value into the entry shape (string -> int)",
      root: { a: "x" },
      path: "$.a",
      value: "42",
      schema: stringIntMapSchema(),
      expectedCount: 1,
      expectedRoot: { a: 42 },
    },
    {
      testTitle: "set sequence index replaces in place",
      root: [1, 2],
      path: "$[0]",
      value: 9,
      expectedCount: 1,
      expectedRoot: [9, 2],
    },
    {
      testTitle: "set at index == length appends",
      root: [1],
      path: "$[1]",
      value: 2,
      expectedCount: 1,
      expectedRoot: [1, 2],
    },
    {
      // go: "Set slice index with expansion" — missing slots default to null
      testTitle: "set past the end expands with defaults (go expansion)",
      root: [0],
      path: "$[3]",
      value: 10,
      expectedCount: 1,
      expectedRoot: [0, null, null, 10],
    },
    {
      testTitle: "set class-instance field via schema (go auto-init struct)",
      root: new UserProfile(),
      path: "$.Name",
      value: "amy",
      schema: userProfileSchema(),
      expectedCount: 1,
      expectedRoot: new UserProfile({ Name: "amy" }),
    },
    {
      // schema-driven creation of a nested struct + nilable field
      testTitle: "set creates nested struct and field via schema (go implicit creation)",
      root: new UserProfile(),
      path: "$.Address.ZipCode",
      value: "1234",
      schema: userProfileSchema(),
      expectedCount: 1,
      expectedRoot: new UserProfile({ Address: new Address({ ZipCode: "1234" }) }),
    },
    {
      testTitle: "union set on class-instance fields sets both (go $.User['Name','Email'])",
      root: new ComplexData(),
      path: "$.User['Name','Email']",
      value: "x@y.z",
      expectedCount: 2,
      expectedRoot: new ComplexData({
        User: new User({ Name: "x@y.z", Email: "x@y.z" }),
      }),
    },
    {
      testTitle: "slice selector sets every second element (go $[::2])",
      root: [1, 2, 3, 4],
      path: "$[::2]",
      value: 0,
      expectedCount: 2,
      expectedRoot: [0, 2, 0, 4],
    },
    {
      testTitle: "recursive descent set updates every Name (go $..Name)",
      root: [
        { User: { Name: "Alice" } },
        { Items: [{ Name: "Item 1" }] },
      ],
      path: "$..Name",
      value: "OneName",
      expectedCount: 2,
      expectedRoot: [
        { User: { Name: "OneName" } },
        { Items: [{ Name: "OneName" }] },
      ],
    },
    {
      // root set on a container replaces it in place (primitive roots need DynObject)
      testTitle: "set root replaces a container source in place (go Set root object)",
      root: { a: 1 },
      path: "$",
      value: { b: 2 },
      expectedCount: 1,
      expectedRoot: { b: 2 },
    },
    {
      // go: Set type mismatch — with a schema, the value must convert
      testTitle: "setting an object into an int-valued map entry errors (go type mismatch)",
      root: { a: 1 },
      path: "$.a",
      value: { x: 1 },
      schema: stringIntMapSchema(),
      expectedCount: 0,
      expectedRoot: { a: 1 },
      expectError: "conversion failed",
    },
    {
      testTitle: "union set with negative index member sets first and last",
      root: [1, 2, 3],
      path: "$[0,-1]",
      value: 99,
      expectedCount: 2,
      expectedRoot: [99, 2, 99],
    },
    {
      testTitle: "union set with slice member sets the expanded indices",
      root: [0, 1, 2, 3, 4],
      path: "$[1, 2:4]",
      value: 9,
      expectedCount: 3,
      expectedRoot: [0, 9, 9, 9, 4],
    },
    // --- native built-in shapes -------------------------------------------
    {
      testTitle: "plain object set existing entry",
      root: { a: 1 },
      path: "$.a",
      value: 5,
      expectedCount: 1,
      expectedRoot: { a: 5 },
    },
    {
      testTitle: "plain object set creates a missing entry",
      root: {},
      path: "$.new_key",
      value: 7,
      expectedCount: 1,
      expectedRoot: { new_key: 7 },
    },
    {
      testTitle: "array of class instances: set field through index",
      root: [
        new User({ ID: 1, Name: "Ann", Email: "a@x" }),
        new User({ ID: 2, Name: "Bob", Email: "b@x" }),
      ],
      path: "$[0].Name",
      value: "X",
      expectedCount: 1,
      expectedRoot: [
        new User({ ID: 1, Name: "X", Email: "a@x" }),
        new User({ ID: 2, Name: "Bob", Email: "b@x" }),
      ],
    },
    {
      testTitle: "tuple-analogue array positional element set",
      root: [1, "x", true],
      path: "$[1]",
      value: "y",
      expectedCount: 1,
      expectedRoot: [1, "y", true],
    },
    {
      testTitle: "set into a defaulted nested class instance",
      root: new UserWithAddress(),
      path: "$.Address.City",
      value: "Mombasa",
      expectedCount: 1,
      expectedRoot: new UserWithAddress({ Address: new Address({ City: "Mombasa" }) }),
    },
  ];
}

test("set_cases", () => {
  const report = new TestReport(
    "set_cases",
    "set at a JSONPath with schema-driven coercion/creation (src/application/set.ts)",
  );
  const tokenizer = new RegexPathTokenizer();
  for (const testCase of yieldTestData()) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const root = reflectClone(testCase.root);
    const segments = parsePath(tokenizer, testCase.path);
    let countValue: number | undefined;
    let errorMessage = "";
    try {
      countValue = set(root, segments, testCase.value, testCase.schema);
    } catch (err) {
      errorMessage = err instanceof Error ? err.message : String(err);
    }
    if (testCase.expectError !== undefined) {
      if (countValue !== undefined || !errorMessage.includes(testCase.expectError)) {
        report.failCase(
          `expected error containing ${JSON.stringify(testCase.expectError)}, got ${countValue !== undefined ? `Ok(${countValue})` : JSON.stringify(errorMessage)}`,
        );
      }
      continue;
    }
    if (countValue === undefined) {
      report.failCase(`unexpected error: ${errorMessage}`);
      continue;
    }
    if (countValue !== testCase.expectedCount) {
      report.failCase(`path ${testCase.path} || expected count ${testCase.expectedCount}, got ${countValue}`);
    }
    if (!areEqual(root, testCase.expectedRoot) && !eq(root, testCase.expectedRoot)) {
      report.failCase(
        `path ${testCase.path} || expected root ${render(testCase.expectedRoot)}, got ${render(root)}`,
      );
    }
  }
  report.finish();
});
