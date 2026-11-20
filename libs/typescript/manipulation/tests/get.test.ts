// Data-driven suite for src/application/get.ts (ADR 0004/0009): cases ported
// from libs/rust/manipulation/tests/application/get.rs (itself ported from
// go-json's object/get_test.go), expressed over plain objects, arrays and
// class instances.

import { test } from "vitest";
import { TestReport, newTestTitle } from "@dyn-rt/test-support";
import { RegexPathTokenizer, parsePath } from "@dyn-rt/path";
import { areEqual, get } from "../src/index.js";
import { ComplexData, ComplexDataItem, User, UserProfile, UserWithAddress, Address, eq, render } from "./common.js";

interface GetCase {
  testTitle?: string;
  root: unknown;
  path: string;
  /** expected results, compared with areEqual (deep comparison, like go) */
  expectedValues: unknown[];
  /** when set, get must throw a ManipulationError containing this fragment */
  expectError?: string;
}

function yieldTestData(): GetCase[] {
  return [
    {
      // go case 1: complex nested path with maps
      testTitle: "nested map path",
      root: { data: { metadata: "placeholder" } },
      path: "$.data.metadata",
      expectedValues: ["placeholder"],
    },
    {
      // nested class field (go's anonymous struct case)
      testTitle: "class-instance field path (go case 1 equivalent)",
      root: new UserProfile({
        Name: "amy",
        Age: 30,
        Address: new Address({ Street: "123 Main St", City: "Anytown", ZipCode: null }),
      }),
      path: "$.Address.City",
      expectedValues: ["Anytown"],
    },
    {
      // go case 2: recursive descent over a mixed slice
      testTitle: "recursive descent collects all Name fields (go case 2)",
      root: [{ User: { Name: "Alice" } }, { User: { Name: "Bob" } }, { Items: [{ Name: "Item 1" }, { Name: "Item 2" }] }],
      path: "$..Name",
      expectedValues: ["Alice", "Bob", "Item 1", "Item 2"],
    },
    {
      // go case 3: union on a class
      testTitle: "union operator over class-instance fields (go case 3)",
      root: new ComplexData({
        User: new User({ ID: 10, Name: "Charlie", Email: "charlie@example.com" }),
      }),
      path: "$.User['ID', 'Name']",
      expectedValues: [10, "Charlie"],
    },
    {
      // go case 4: deep path with index and field
      testTitle: "array index then class-instance field (go case 4)",
      root: new ComplexData({
        Items: [new ComplexDataItem({ Name: "First", Value: 10 }), new ComplexDataItem({ Name: "Second", Value: 20 }), new ComplexDataItem({ Name: "Third", Value: 30 })],
      }),
      path: "$.Items[1].Value",
      expectedValues: [20],
    },
    {
      // go case 5: path that matches nothing
      testTitle: "missing map key yields no results (go case 5)",
      root: { store: { book: "fiction" } },
      path: "$.store.magazine",
      expectedValues: [],
    },
    {
      // go case 6 equivalent: recursive descent + slice + union
      testTitle: "recursive descent with slice then union (go case 6 equivalent)",
      root: [{ one: { Three: [0, 1, 2, 3, { "04": [0, 4] }, 5] } }, { Two: { Three: [0, 3, { TwentyFour: 24 }] } }],
      path: "$..Three[::2]['TwentyFour','04']",
      expectedValues: [[0, 4], 24],
    },
    {
      // go $.one..Two[*]: descent into key then wildcard over array elements
      testTitle: "descent into key then wildcard over array (go $.one..Two[*])",
      root: {
        one: [1, { Two: [12] }, { three: ["four"] }, { five: [{ Two: [13] }] }],
      },
      path: "$.one..Two[*]",
      expectedValues: [12, 13],
    },
    {
      // go $.three[1].['1']: index into array element by bracket key
      testTitle: "index then bracket key (go $.three[1].['1'])",
      root: { three: [{ "0": 0 }, { "1": 1 }] },
      path: "$.three[1].['1']",
      expectedValues: [1],
    },
    {
      testTitle: "get root value (go)",
      root: "root value",
      path: "$",
      expectedValues: ["root value"],
    },
    {
      testTitle: "index out of bounds errors (go)",
      root: [1, 2],
      path: "$[5]",
      expectedValues: [],
      expectError: "out of range",
    },
    {
      testTitle: "key on primitive errors (go)",
      root: 123,
      path: "$.key",
      expectedValues: [],
      expectError: "unsupported",
    },
    {
      // TS object keys are always strings, so the int-keyed-map cases become:
      // an index segment on a map is unsupported
      testTitle: "index segment on a map errors (TS keys are strings)",
      root: { "7": "seven" },
      path: "$[7]",
      expectedValues: [],
      expectError: "unsupported",
    },
    {
      testTitle: "bracket key reads a string-keyed map entry (go map key case)",
      root: { "3": "three" },
      path: "$['3']",
      expectedValues: ["three"],
    },
    {
      testTitle: "slice selector [1:5:2]",
      root: [0, 1, 2, 3, 4, 5],
      path: "$[1:5:2]",
      expectedValues: [1, 3],
    },
    {
      testTitle: "wildcard over array",
      root: [1, 2, 3],
      path: "$[*]",
      expectedValues: [1, 2, 3],
    },
    {
      testTitle: "union of indices",
      root: [10, 20, 30],
      path: "$[0,2]",
      expectedValues: [10, 30],
    },
    {
      testTitle: "negative index counts from the end",
      root: [10, 20, 30],
      path: "$[-1]",
      expectedValues: [30],
    },
    {
      testTitle: "Some-enum payload is the root value",
      root: { variant: "Some", payload: 42 },
      path: "$",
      expectedValues: [42],
    },
    {
      testTitle: "Some-enum unwrap on traversal",
      root: { variant: "Some", payload: { a: 1 } },
      path: "$.a",
      expectedValues: [1],
    },
    {
      testTitle: "union with negative index member",
      root: [10, 20, 30],
      path: "$[0,-1]",
      expectedValues: [10, 30],
    },
    {
      testTitle: "union with a slice member expands the slice",
      root: [0, 1, 2, 3, 4, 5],
      path: "$[1, 2:4]",
      expectedValues: [1, 2, 3],
    },
    {
      testTitle: "union of key and wildcard over a map",
      root: { a: 1, b: 2 },
      path: "$['a', *]",
      expectedValues: [1, 1, 2],
    },
    // --- native built-in shapes (arrays of class instances, nested combos) --
    {
      testTitle: "array of class instances: index then field",
      root: [new User({ ID: 1, Name: "Ann", Email: "a@x" }), new User({ ID: 2, Name: "Bob", Email: "b@x" })],
      path: "$[1].Name",
      expectedValues: ["Bob"],
    },
    {
      testTitle: "map of string to array of class instances: deep path",
      root: { users: [new User({ ID: 1, Name: "Ann", Email: "a@x" })] },
      path: "$.users[0].Name",
      expectedValues: ["Ann"],
    },
    {
      testTitle: "nested class instance field",
      root: new UserWithAddress({
        Name: "Ann",
        Address: new Address({ Street: "1 Main", City: "Nairobi", ZipCode: "00100" }),
      }),
      path: "$.Address.City",
      expectedValues: ["Nairobi"],
    },
    {
      // tuples are arrays in TS: positional access via index
      testTitle: "array positional element get (tuple analogue)",
      root: [1, "x", true],
      path: "$[1]",
      expectedValues: ["x"],
    },
    {
      testTitle: "array negative index get",
      root: [10, 20, 30],
      path: "$[-1]",
      expectedValues: [30],
    },
    {
      testTitle: "recursive descent over array of class instances",
      root: [new ComplexDataItem({ Name: "First", Value: 1 }), new ComplexDataItem({ Name: "Second", Value: 2 })],
      path: "$..Name",
      expectedValues: ["First", "Second"],
    },
  ];
}

test("get_cases", () => {
  const report = new TestReport("get_cases", "get over values at a JSONPath (src/application/get.ts)");
  const tokenizer = new RegexPathTokenizer();
  for (const testCase of yieldTestData()) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const segments = parsePath(tokenizer, testCase.path);
    let values: unknown[] | undefined;
    let errorMessage = "";
    try {
      values = get(testCase.root, segments);
    } catch (err) {
      errorMessage = err instanceof Error ? err.message : String(err);
    }
    if (testCase.expectError !== undefined) {
      if (values !== undefined || !errorMessage.includes(testCase.expectError)) {
        report.failCase(`expected error containing ${JSON.stringify(testCase.expectError)}, got ${values !== undefined ? `${values.length} results` : JSON.stringify(errorMessage)}`);
      }
      continue;
    }
    if (values === undefined) {
      report.failCase(`unexpected error: ${errorMessage}`);
      continue;
    }
    if (values.length !== testCase.expectedValues.length) {
      report.failCase(`result count: expected ${testCase.expectedValues.length}, got ${values.length} (${values.map(render).join(", ")})`);
      continue;
    }
    for (let i = 0; i < values.length; i++) {
      if (!areEqual(values[i], testCase.expectedValues[i]) && !eq(values[i], testCase.expectedValues[i])) {
        report.failCase(`result ${i}: expected ${render(testCase.expectedValues[i])}, got ${render(values[i])}`);
        break;
      }
    }
  }
  report.finish();
});
