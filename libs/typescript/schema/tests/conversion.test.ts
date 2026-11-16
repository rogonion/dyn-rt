// Data-driven suite for src/application/conversion.ts (ADR 0004/0009) — TS
// port of libs/rust/schema/tests/application/conversion.rs: converting data
// into the shape a schema dictates. Unlike Rust (which converts into a
// `&mut dyn Reflect` target in place), the TS `convert` builds and returns the
// converted value; a `target` is passed only to preserve its prototype on
// struct shapes.

import { test } from "vitest";
import { TestReport, newTestTitle } from "@dyn-rt/test-support";
import {
  convertWith,
  enumNode,
  jsonScalar,
  nilable,
  scalar,
  schemaSet,
  type ConversionOptions,
  type Schema,
} from "../src/index.js";
import {
  Circle,
  Square,
  User,
  UserProfile,
  UserWithAddress,
  employeeSchema,
  eq,
  listOfProductsSchema,
  mapUserSchema,
  productSchema,
  sampleUser,
  shapeSchema,
  userProfileSchema,
  userSchema,
  userValue,
  userWithAddressSchema,
} from "./common.js";

interface ConversionCase {
  testTitle?: string;
  source: unknown;
  schema: Schema;
  target?: unknown;
  options?: ConversionOptions;
  /** { value, validSetKeys, instanceOf? } or { error: fragment expected in the message } */
  expected:
    | { value: unknown; validSetKeys: string[]; instanceOf?: new () => unknown }
    | { error: string };
}

function yieldTestData(): ConversionCase[] {
  return [
    {
      testTitle: "string coerces to number",
      source: "42",
      schema: scalar("i64"),
      expected: { value: 42, validSetKeys: [] },
    },
    {
      testTitle: "number renders to string",
      source: 5,
      schema: scalar("string"),
      expected: { value: "5", validSetKeys: [] },
    },
    {
      testTitle: "non-zero number coerces to bool true",
      source: 1,
      schema: scalar("bool"),
      expected: { value: true, validSetKeys: [] },
    },
    {
      testTitle: "string parses to float",
      source: "2.5",
      schema: scalar("f64"),
      expected: { value: 2.5, validSetKeys: [] },
    },
    {
      testTitle: "json number converts to int",
      source: 7,
      schema: scalar("i64"),
      expected: { value: 7, validSetKeys: [] },
    },
    {
      testTitle: "json string round-trips through a json string node",
      source: "x",
      schema: jsonScalar("string"),
      expected: { value: "x", validSetKeys: [] },
    },
    {
      testTitle: "plain object converts into a struct shape field by field, keeping the target's prototype",
      source: userValue(),
      schema: userSchema(),
      target: new User(),
      expected: { value: { ID: 1, Name: "amy", Email: "amy@example.com" }, validSetKeys: [], instanceOf: User },
    },
    {
      testTitle: "class instance source converts into a struct shape",
      source: sampleUser(),
      schema: userSchema(),
      target: new User(),
      expected: { value: { ID: 1, Name: "amy", Email: "amy@example.com" }, validSetKeys: [], instanceOf: User },
    },
    {
      testTitle: "nested plain object converts into nested structs (incl. the Option-like field)",
      source: {
        Name: "amy",
        Age: 30,
        Address: { Street: "1 Main St", City: "Springfield", ZipCode: "12345" },
      },
      schema: userProfileSchema(),
      target: new UserProfile(),
      expected: {
        value: {
          Name: "amy",
          Age: 30,
          Address: { Street: "1 Main St", City: "Springfield", ZipCode: { variant: "Some", payload: "12345" } },
        },
        validSetKeys: [],
        instanceOf: UserProfile,
      },
    },
    {
      testTitle: "array converts into a sequence of structs",
      source: [{ ID: 7, Name: "widget", Price: 9.99 }],
      schema: listOfProductsSchema(),
      expected: { value: [{ ID: 7, Name: "widget", Price: 9.99 }], validSetKeys: [] },
    },
    {
      // TS object keys are strings; the int key schema coerces "1" -> 1 -> "1"
      testTitle: "object converts into a map (string key coerced via the key schema, value through indirection)",
      source: { "1": userValue() },
      schema: mapUserSchema(),
      expected: { value: { "1": { ID: 1, Name: "amy", Email: "amy@example.com" } }, validSetKeys: [] },
    },
    {
      testTitle: "indirection field converts straight through (null zip stays null via nilable)",
      source: {
        Name: "amy",
        Address: { Street: "1 Main St", City: "Springfield", ZipCode: null },
      },
      schema: userWithAddressSchema(),
      target: new UserWithAddress(),
      expected: {
        value: { Name: "amy", Address: { Street: "1 Main St", City: "Springfield", ZipCode: null } },
        validSetKeys: [],
        instanceOf: UserWithAddress,
      },
    },
    {
      testTitle: "polymorphic set converts via the matching node and reports the key",
      source: userValue(),
      schema: schemaSet("User", [
        ["User", userSchema()],
        ["Product", productSchema()],
      ]),
      target: new User(),
      expected: { value: { ID: 1, Name: "amy", Email: "amy@example.com" }, validSetKeys: ["User"], instanceOf: User },
    },
    {
      testTitle: "shape set converts circle data via the Circle node and reports the key",
      source: { Radius: 2.5 },
      schema: shapeSchema(),
      target: new Circle(),
      expected: { value: { Radius: 2.5 }, validSetKeys: ["Circle"], instanceOf: Circle },
    },
    {
      // plain-object sources are strict: "Side" has no schema field in the
      // Circle node, so conversion falls through to the Square node
      testTitle: "shape set falls through to the Square node for square data",
      source: { Side: 3.0 },
      schema: shapeSchema(),
      target: new Square(),
      expected: { value: { Side: 3.0 }, validSetKeys: ["Square"], instanceOf: Square },
    },
    {
      // the TS analogue of the Rust "dyn Shape" trait target: the target is a
      // class instance, and the result keeps its prototype
      testTitle: "shape set result keeps the target class prototype (the dyn Shape analogue)",
      source: { Radius: 4.5 },
      schema: shapeSchema(),
      target: new Circle(),
      expected: { value: { Radius: 4.5 }, validSetKeys: ["Circle"], instanceOf: Circle },
    },
    {
      testTitle: "employee converts with nested struct, list and map fields",
      source: {
        ID: 3,
        Profile: { Name: "amy", Age: 30, Address: { Street: "1 Main St", City: "Springfield", ZipCode: "12345" } },
        Skills: ["rust", "go"],
        ProjectHours: { core: 40 },
      },
      schema: employeeSchema(),
      expected: {
        value: {
          ID: 3,
          Profile: {
            Name: "amy",
            Age: 30,
            Address: { Street: "1 Main St", City: "Springfield", ZipCode: { variant: "Some", payload: "12345" } },
          },
          Skills: ["rust", "go"],
          ProjectHours: { core: 40 },
        },
        validSetKeys: [],
      },
    },
    {
      testTitle: "scalar source cannot convert into a struct shape",
      source: 42,
      schema: userSchema(),
      expected: { error: "unsupported source kind" },
    },
    {
      testTitle: "null source against a nilable node returns the target as-is",
      source: null,
      schema: nilable(enumNode([["Some", scalar("string")], ["None", undefined]])),
      target: "keep",
      expected: { value: "keep", validSetKeys: [] },
    },
    {
      testTitle: "null source against a non-nilable node errors",
      source: null,
      schema: enumNode([["Some", scalar("string")], ["None", undefined]]),
      expected: { error: "source cannot be nil" },
    },
  ];
}

test("conversion_cases", () => {
  const report = new TestReport(
    "conversion_cases",
    "Data-driven suite for src/application/conversion.ts (ADR 0004/0009).",
  );
  for (const testCase of yieldTestData()) {
    report.beginCase(newTestTitle(testCase.testTitle));
    let result: { value: unknown; validSetKeys: string[] } | undefined;
    let errorMessage = "";
    try {
      result = convertWith(testCase.source, testCase.schema, testCase.target, testCase.options ?? {});
    } catch (err) {
      errorMessage = err instanceof Error ? err.message : String(err);
    }
    if ("error" in testCase.expected) {
      if (result !== undefined || !errorMessage.includes(testCase.expected.error)) {
        report.failCase(
          `expected error containing ${JSON.stringify(testCase.expected.error)}, got ${result !== undefined ? `value ${JSON.stringify(result.value)}` : JSON.stringify(errorMessage)}`,
        );
      }
    } else if (result === undefined) {
      report.failCase(`expected ok, got error: ${errorMessage}`);
    } else {
      if (!eq(result.value, testCase.expected.value)) {
        report.failCase(`value: expected ${JSON.stringify(testCase.expected.value)}, got ${JSON.stringify(result.value)}`);
      }
      if (!eq(result.validSetKeys, testCase.expected.validSetKeys)) {
        report.failCase(
          `valid set keys: expected ${JSON.stringify(testCase.expected.validSetKeys)}, got ${JSON.stringify(result.validSetKeys)}`,
        );
      }
      if (testCase.expected.instanceOf !== undefined && !(result.value instanceof testCase.expected.instanceOf)) {
        report.failCase(`expected an instance of ${testCase.expected.instanceOf.name}, got ${result.value?.constructor?.name}`);
      }
    }
  }

  // custom converter registry case (go's WithCustomConverters, keyed by the
  // source's runtime kind — "number" here): doubles any numeric source
  const doubleNumbers: ConversionOptions = {
    customConverters: {
      number: (data) => (data as number) * 2,
    },
  };
  report.beginCase(newTestTitle("custom: custom converter doubles a number source"));
  try {
    const result = convertWith(5, scalar("i64"), undefined, doubleNumbers);
    if (result.value !== 10) {
      report.failCase(`expected 10, got ${JSON.stringify(result.value)}`);
    }
  } catch (err) {
    report.failCase(`expected ok, got error: ${err instanceof Error ? err.message : String(err)}`);
  }

  report.finish();
});
