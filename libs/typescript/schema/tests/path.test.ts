// Data-driven suite for src/application/path.ts: schema-path resolution from
// parsed JSONPath segments (ADR 0004/0009) — TS port of
// libs/rust/schema/tests/application/path.rs.

import { test } from "vitest";
import { TestReport, newTestTitle } from "@dyn-rt/test-support";
import { RegexPathTokenizer, parsePath } from "@dyn-rt/path";
import { schemaAtPath, type Schema, type SchemaShape } from "../src/index.js";
import { addressSchema, companySchema, listOfProductsSchema, listOfShapesSchema, nestedItemSchema, userProfileSchema, userSchema } from "./common.js";

interface PathCase {
  testTitle?: string;
  path: string;
  schema: Schema;
  /** { kind, typeName, validSetKeys } or { error: fragment expected in the message } */
  expected: { kind: SchemaShape["kind"]; typeName: string; validSetKeys: string[] } | { error: string };
}

function yieldTestData(): PathCase[] {
  return [
    {
      testTitle: "root resolves to the schema root node",
      path: "$",
      schema: userSchema(),
      expected: { kind: "Class", typeName: "User", validSetKeys: [] },
    },
    {
      testTitle: "class field resolves to its scalar node",
      path: "$.Name",
      schema: userSchema(),
      expected: { kind: "Scalar", typeName: "string", validSetKeys: [] },
    },
    {
      testTitle: "nested class field path",
      path: "$.Address.Street",
      schema: userProfileSchema(),
      expected: { kind: "Scalar", typeName: "string", validSetKeys: [] },
    },
    {
      testTitle: "sequence index through an indirection into a class field",
      path: "$.Employees[0].Name",
      schema: companySchema(),
      expected: { kind: "Scalar", typeName: "string", validSetKeys: [] },
    },
    {
      testTitle: "map-typed field resolves to the map node",
      path: "$.MapData",
      schema: nestedItemSchema(),
      expected: { kind: "Map", typeName: "map", validSetKeys: [] },
    },
    {
      testTitle: "Option-like field resolves to the enum node",
      path: "$.ZipCode",
      schema: addressSchema(),
      expected: { kind: "Enum", typeName: "enum", validSetKeys: [] },
    },
    {
      testTitle: "sequence index resolves to the element schema",
      path: "$[0]",
      schema: listOfProductsSchema(),
      expected: { kind: "Class", typeName: "Product", validSetKeys: [] },
    },
    {
      // the set's default key "default" is absent from the nodes, so the
      // first node (Circle) is used and its key recorded
      testTitle: "set-valued element resolves to its default/first node and records the key",
      path: "$[0]",
      schema: listOfShapesSchema(),
      expected: { kind: "Class", typeName: "Circle", validSetKeys: ["Circle"] },
    },
    {
      testTitle: "unknown class field errors",
      path: "$.Unknown",
      schema: userSchema(),
      expected: { error: "not found" },
    },
  ];
}

test("schema_path_cases", () => {
  const report = new TestReport("schema_path_cases", "Data-driven suite for src/application/path.ts: schema-path resolution.");
  const tokenizer = new RegexPathTokenizer();
  for (const testCase of yieldTestData()) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const parsed = parsePath(tokenizer, testCase.path);
    let resolved: { node: { shape: SchemaShape; typeName: string }; validSetKeys: string[] } | undefined;
    let errorMessage = "";
    try {
      resolved = schemaAtPath(parsed[0]!, testCase.schema);
    } catch (err) {
      errorMessage = err instanceof Error ? err.message : String(err);
    }
    if ("error" in testCase.expected) {
      if (resolved !== undefined || !errorMessage.includes(testCase.expected.error)) {
        report.failCase(`expected error containing ${JSON.stringify(testCase.expected.error)}, got ${resolved !== undefined ? `kind ${resolved.node.shape.kind}` : JSON.stringify(errorMessage)}`);
      }
    } else if (resolved === undefined) {
      report.failCase(`expected ok, got error: ${errorMessage}`);
    } else {
      const kind = resolved.node.shape.kind;
      const typeName = resolved.node.typeName;
      const keys = resolved.validSetKeys;
      if (kind !== testCase.expected.kind || typeName !== testCase.expected.typeName || !eqArrays(keys, testCase.expected.validSetKeys)) {
        report.failCase(`expected (${testCase.expected.kind}, ${testCase.expected.typeName}, ${JSON.stringify(testCase.expected.validSetKeys)}), got (${kind}, ${typeName}, ${JSON.stringify(keys)})`);
      }
    }
  }
  report.finish();
});

function eqArrays(a: string[], b: string[]): boolean {
  return a.length === b.length && a.every((value, index) => value === b[index]);
}
