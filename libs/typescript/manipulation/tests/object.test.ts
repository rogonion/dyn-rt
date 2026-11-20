// Data-driven suite for src/application/object.ts (ADR 0004/0009): the
// DynObject entrypoint wrapper — text vs parsed-segment path inputs, read-only
// vs held source, schema attachment, get/set/delete/forEach (port of
// libs/rust/manipulation/tests/application/object.rs).

import { test } from "vitest";
import { TestReport, newTestTitle } from "@dyn-rt/test-support";
import { RegexPathTokenizer, parsePath } from "@dyn-rt/path";
import { DynObject, areEqual } from "../src/index.js";
import { eq, render, userProfileSchema, UserProfile } from "./common.js";

function valueSource(): unknown {
  return { a: { b: 1, c: [2, 3] }, name: "Alice" };
}

function sourceWithName(name: string): unknown {
  return { a: { b: 1, c: [2, 3] }, name };
}

function sourceWithoutAB(): unknown {
  return { a: { c: [2, 3] }, name: "Alice" };
}

type Op = { kind: "get" } | { kind: "set"; value: unknown } | { kind: "delete" } | { kind: "forEach" };

interface ObjectCase {
  testTitle?: string;
  makeSource: () => unknown;
  path: string;
  op: Op;
  expectedValues?: unknown[];
  expectedSource?: unknown;
  expectedVisited?: string;
  expectError?: string;
}

function yieldTestData(): ObjectCase[] {
  return [
    {
      testTitle: "get with a text path",
      makeSource: valueSource,
      path: "$.a.b",
      op: { kind: "get" },
      expectedValues: [1],
    },
    {
      testTitle: "get with a wildcard text path",
      makeSource: valueSource,
      path: "$.a.c[*]",
      op: { kind: "get" },
      expectedValues: [2, 3],
    },
    {
      testTitle: "set with a text path updates the source",
      makeSource: valueSource,
      path: "$.name",
      op: { kind: "set", value: "Bob" },
      expectedSource: sourceWithName("Bob"),
    },
    {
      testTitle: "delete with a text path removes the member",
      makeSource: valueSource,
      path: "$.a.b",
      op: { kind: "delete" },
      expectedSource: sourceWithoutAB(),
    },
    {
      testTitle: "for_each visits every wildcard match",
      makeSource: valueSource,
      path: "$.a.c[*]",
      op: { kind: "forEach" },
      expectedVisited: "2,3",
    },
  ];
}

test("dyn_object_cases", () => {
  const report = new TestReport("dyn_object_cases", "DynObject wrapper: path input reps + get/set/delete/forEach (src/application/object.ts)");
  for (const testCase of yieldTestData()) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const source = testCase.makeSource();
    const obj = DynObject.new(source);
    let errorMessage = "";
    try {
      switch (testCase.op.kind) {
        case "get": {
          const found = obj.get(testCase.path);
          if (testCase.expectedValues === undefined || found.length !== testCase.expectedValues.length) {
            errorMessage = `expected ${testCase.expectedValues?.length} value(s), got ${found.length}`;
            break;
          }
          for (let i = 0; i < found.length; i++) {
            if (!areEqual(found[i], testCase.expectedValues[i]) && !eq(found[i], testCase.expectedValues[i])) {
              errorMessage = `value ${i}: expected ${render(testCase.expectedValues[i])}, got ${render(found[i])}`;
              break;
            }
          }
          break;
        }
        case "set":
          obj.set(testCase.path, testCase.op.value);
          break;
        case "delete":
          obj.delete(testCase.path);
          break;
        case "forEach": {
          const visited: string[] = [];
          obj.forEach(testCase.path, (value) => {
            visited.push(String(value));
            return true;
          });
          if (visited.join(",") !== testCase.expectedVisited) {
            errorMessage = `visited ${JSON.stringify(visited.join(","))}`;
          }
          break;
        }
      }
    } catch (err) {
      errorMessage = err instanceof Error ? err.message : String(err);
    }
    if (testCase.expectError !== undefined) {
      if (!errorMessage.includes(testCase.expectError)) {
        report.failCase(`expected error containing ${JSON.stringify(testCase.expectError)}, got ${JSON.stringify(errorMessage)}`);
      }
      continue;
    }
    if (errorMessage !== "") {
      report.failCase(errorMessage);
      continue;
    }
    if (testCase.expectedSource !== undefined) {
      if (!areEqual(source, testCase.expectedSource) && !eq(source, testCase.expectedSource)) {
        report.failCase(`source mismatch: expected ${render(testCase.expectedSource)}, got ${render(source)}`);
      }
    }
  }
  report.finish();
});

test("dyn_object_path_input_reps", () => {
  const report = new TestReport("dyn_object_path_input_reps", "DynObject accepts text, a parsed segment, or parsed segments (src/application/object.ts)");
  const tokenizer = new RegexPathTokenizer();
  const parsed = parsePath(tokenizer, "$.a.b");

  report.beginCase(newTestTitle("get via parsed segments matches get via text"));
  {
    const source = valueSource();
    const obj = DynObject.new(source);
    const viaSegments = obj.get(parsed);
    const viaText = obj.get("$.a.b");
    const viaSegment = obj.get(parsed[0]!);
    if (
      viaSegments.length !== 1 ||
      viaText.length !== 1 ||
      viaSegment.length !== 1 ||
      !areEqual(viaSegments[0], viaText[0]) ||
      !areEqual(viaSegments[0], viaSegment[0]) ||
      !areEqual(viaSegments[0], 1)
    ) {
      report.failCase("text / segment / segments gets disagree");
    }
  }

  report.beginCase(newTestTitle("set via a parsed segment matches set via text"));
  {
    const a = valueSource();
    const b = valueSource();
    DynObject.new(a).set(parsed[0]!, 9);
    DynObject.new(b).set("$.a.b", 9);
    if (!areEqual(a, b)) {
      report.failCase(`segment-set and text-set disagree: ${render(a)} vs ${render(b)}`);
    }
  }
  report.finish();
});

test("dyn_object_source_variants", () => {
  const report = new TestReport("dyn_object_source_variants", "DynObject source handling / root replacement (src/application/object.ts)");

  report.beginCase(newTestTitle("get works on the wrapped source"));
  {
    const obj = DynObject.new(valueSource());
    const found = obj.get("$.name");
    if (found.length !== 1 || !areEqual(found[0], "Alice")) {
      report.failCase(`expected ["Alice"], got ${found.map(render).join(",")}`);
    }
  }

  report.beginCase(newTestTitle("set mutates the wrapped source in place"));
  {
    const source = valueSource();
    const obj = DynObject.new(source);
    const count = obj.set("$.name", "Carol");
    if (count !== 1) {
      report.failCase(`expected 1 set, got ${count}`);
    }
    if (!areEqual(obj.getSource(), sourceWithName("Carol")) || !areEqual(source, sourceWithName("Carol"))) {
      report.failCase(`source was not modified as expected: ${render(source)}`);
    }
  }

  report.beginCase(newTestTitle("root set replaces a primitive source (only possible via DynObject)"));
  {
    const obj = DynObject.new(1);
    const count = obj.set("$", "root");
    if (count !== 1 || obj.getSource() !== "root") {
      report.failCase(`expected root replaced with "root", got ${render(obj.getSource())}`);
    }
  }

  report.beginCase(newTestTitle("root delete resets a primitive source to its zero"));
  {
    const obj = DynObject.new(123);
    const count = obj.delete("$");
    if (count !== 1 || obj.getSource() !== 0) {
      report.failCase(`expected root reset to 0, got ${render(obj.getSource())}`);
    }
  }

  report.beginCase(newTestTitle("withSchemaNode attaches a schema used by set"));
  {
    // userProfileSchema expects Name: string — setting a number coerces to "42"
    const obj = DynObject.new(new UserProfile()).withSchemaNode(userProfileSchema());
    const count = obj.set("$.Name", 42);
    const source = obj.getSource() as { Name: unknown };
    if (count !== 1 || source.Name !== "42") {
      report.failCase(`expected Name coerced to "42", got ${render(source.Name)}`);
    }
  }

  report.finish();
});
