// Application suite for src/application/for_each.ts: callback per value found
// at a JSONPath, incl. early stop (port of
// libs/rust/manipulation/tests/application/for_each.rs).

import { test } from "vitest";
import { TestReport, newTestTitle } from "@dyn-rt/test-support";
import { RegexPathTokenizer, parsePath } from "@dyn-rt/path";
import { areEqual, forEach } from "../src/index.js";
import { ComplexDataItem, render } from "./common.js";

interface ForEachCase {
  testTitle?: string;
  root: unknown;
  path: string;
  /** expected visited values, compared with areEqual (deep comparison) */
  expectedVisited: unknown[];
  /** when set, the callback returns false after this many visits (early stop) */
  stopAfter?: number;
}

function yieldTestData(): ForEachCase[] {
  return [
    {
      testTitle: "for_each visits every wildcard match",
      root: { one: [1, 2], two: 3 },
      path: "$[*]",
      expectedVisited: [[1, 2], 3],
    },
    {
      testTitle: "for_each over union with negative index member",
      root: [10, 20, 30],
      path: "$[0,-1]",
      expectedVisited: [10, 30],
    },
    {
      testTitle: "for_each over recursive descent collects all matches",
      root: {
        items: [{ Name: "a" }, { Other: "x" }, { Name: "b" }],
      },
      path: "$..Name",
      expectedVisited: ["a", "b"],
    },
    {
      testTitle: "callback returning false stops iteration early",
      root: [1, 2, 3],
      path: "$[*]",
      expectedVisited: [1],
      stopAfter: 1,
    },
    {
      testTitle: "for_each over a non-matching path visits nothing",
      root: { a: 1 },
      path: "$.missing",
      expectedVisited: [],
    },
    {
      testTitle: "for_each recursive descent over an array of class instances",
      root: [new ComplexDataItem({ Name: "First", Value: 1 }), new ComplexDataItem({ Name: "Second", Value: 2 })],
      path: "$..Name",
      expectedVisited: ["First", "Second"],
    },
    {
      testTitle: "for_each wildcard over a map's values",
      root: { a: 1 },
      path: "$.*",
      expectedVisited: [1],
    },
  ];
}

test("for_each_cases", () => {
  const report = new TestReport("for_each_cases", "forEach at a JSONPath (src/application/for_each.ts)");
  const tokenizer = new RegexPathTokenizer();
  for (const testCase of yieldTestData()) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const segments = parsePath(tokenizer, testCase.path);
    const visited: unknown[] = [];
    try {
      forEach(testCase.root, segments, (value) => {
        visited.push(value);
        if (testCase.stopAfter !== undefined) return visited.length < testCase.stopAfter;
        return true;
      });
    } catch (err) {
      report.failCase(`unexpected error: ${err instanceof Error ? err.message : String(err)}`);
      continue;
    }
    if (visited.length !== testCase.expectedVisited.length) {
      report.failCase(`visited count: expected ${testCase.expectedVisited.length}, got ${visited.length} (${visited.map(render).join(", ")})`);
      continue;
    }
    for (let i = 0; i < visited.length; i++) {
      if (!areEqual(visited[i], testCase.expectedVisited[i])) {
        report.failCase(`visited ${i}: expected ${render(testCase.expectedVisited[i])}, got ${render(visited[i])}`);
        break;
      }
    }
  }
  report.finish();
});
