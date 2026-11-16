// Data-driven suite for src/application/delete.ts (ADR 0004/0009): cases
// ported from libs/rust/manipulation/tests/application/delete.rs (itself
// ported from go-json's object/delete_test.go).

import { test } from "vitest";
import { TestReport, newTestTitle } from "@dyn-rt/test-support";
import { RegexPathTokenizer, parsePath } from "@dyn-rt/path";
import { reflectClone } from "@dyn-rt/reflect";
import { areEqual, deleteAt } from "../src/index.js";
import { ComplexData, User, eq, render } from "./common.js";

interface DeleteCase {
  testTitle?: string;
  root: unknown;
  path: string;
  expectedCount: number;
  /** expected root after the delete, compared with areEqual (deep) */
  expectedRoot: unknown;
  expectError?: string;
}

function yieldTestData(): DeleteCase[] {
  return [
    {
      testTitle: "delete map entry (go $.address)",
      root: { address: { address: "test" } },
      path: "$.address",
      expectedCount: 1,
      expectedRoot: {},
    },
    {
      testTitle: "delete non-existent map key is a no-op (go)",
      root: { a: 1 },
      path: "$.b",
      expectedCount: 0,
      expectedRoot: { a: 1 },
    },
    {
      testTitle: "delete array element shifts left (go $[1])",
      root: [1, 2, 3, 4],
      path: "$[1]",
      expectedCount: 1,
      expectedRoot: [1, 3, 4],
    },
    {
      testTitle: "delete index out of bounds errors (go)",
      root: [1, 2],
      path: "$[5]",
      expectedCount: 0,
      expectedRoot: [1, 2],
      expectError: "out of range",
    },
    {
      testTitle: "delete class-instance field resets it to zero (go)",
      root: new ComplexData({ User: new User({ ID: 10, Name: "amy", Email: "a@b.c" }) }),
      path: "$.User.Name",
      expectedCount: 1,
      expectedRoot: new ComplexData({ User: new User({ ID: 10, Name: "", Email: "a@b.c" }) }),
    },
    {
      testTitle: "union delete on class-instance fields (go)",
      root: new ComplexData({ User: new User({ ID: 10, Name: "amy", Email: "a@b.c" }) }),
      path: "$.User['Name','Email']",
      expectedCount: 2,
      expectedRoot: new ComplexData({ User: new User({ ID: 10 }) }),
    },
    {
      testTitle: "recursive descent delete removes every Name (go $..Name)",
      root: [
        { User: { Name: "Alice" } },
        { Items: [{ Name: "Item 1" }] },
      ],
      path: "$..Name",
      expectedCount: 2,
      expectedRoot: [{ User: {} }, { Items: [{}] }],
    },
    {
      testTitle: "slice selector deletes every second element (go $[::2])",
      root: [1, 2, 3, 4],
      path: "$[::2]",
      expectedCount: 2,
      expectedRoot: [2, 4],
    },
    {
      // root delete on a container resets it in place (primitive roots need DynObject)
      testTitle: "delete root resets a container source to zero (go Delete root object)",
      root: { a: 1 },
      path: "$",
      expectedCount: 1,
      expectedRoot: {},
    },
    {
      testTitle: "delete property from primitive errors (go)",
      root: 123,
      path: "$.key",
      expectedCount: 0,
      expectedRoot: 123,
      expectError: "unsupported",
    },
    {
      testTitle: "union delete with negative index member removes first and last",
      root: [1, 2, 3],
      path: "$[0,-1]",
      expectedCount: 2,
      expectedRoot: [2],
    },
    {
      testTitle: "union delete with slice member removes the expanded indices",
      root: [0, 1, 2, 3, 4],
      path: "$[1, 2:4]",
      expectedCount: 3,
      expectedRoot: [0, 4],
    },
    // --- native built-in shapes -------------------------------------------
    {
      testTitle: "plain object entry delete",
      root: { a: 1, b: 2 },
      path: "$.a",
      expectedCount: 1,
      expectedRoot: { b: 2 },
    },
    {
      testTitle: "array of class instances: element delete shifts left",
      root: [new User({ ID: 1, Name: "Ann", Email: "a@x" }), new User({ ID: 2, Name: "Bob", Email: "b@x" })],
      path: "$[0]",
      expectedCount: 1,
      expectedRoot: [new User({ ID: 2, Name: "Bob", Email: "b@x" })],
    },
    {
      // TS arrays are always resizable (unlike Rust's fixed-size arrays, which
      // error on delete), so an index delete splices
      testTitle: "array index delete splices (TS arrays are resizable)",
      root: [1, "x", true],
      path: "$[0]",
      expectedCount: 1,
      expectedRoot: ["x", true],
    },
  ];
}

test("delete_cases", () => {
  const report = new TestReport("delete_cases", "deleteAt at a JSONPath (src/application/delete.ts)");
  const tokenizer = new RegexPathTokenizer();
  for (const testCase of yieldTestData()) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const root = reflectClone(testCase.root);
    const segments = parsePath(tokenizer, testCase.path);
    let countValue: number | undefined;
    let errorMessage = "";
    try {
      countValue = deleteAt(root, segments);
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
