// Data-driven suite for the schema APPLICATION layer validation
// (src/application/validation.ts) — TS port of
// libs/rust/schema/tests/application/validation.rs (ADR 0004/0009).
// Fixtures mirror go-json's schema/misc_test.go shapes (see common.ts).

import { test } from "vitest";
import { TestReport, newTestTitle } from "@dyn-rt/test-support";
import {
  childrenMustBeValid,
  enumNode,
  jsonScalar,
  mapNode,
  scalar,
  schemaSet,
  sequenceNode,
  validate,
  SchemaError,
  type Schema,
  type SchemaNode,
  type ValidationOptions,
} from "../src/index.js";
import {
  addressSchema,
  circleSchema,
  companySchema,
  employeeSchema,
  jsonMapSchema,
  listOfProductsSchema,
  mapUserSchema,
  nestedItemSchema,
  sampleCompany,
  sampleEmployee,
  sampleNestedItem,
  sampleProduct,
  sampleUser,
  sampleUserProfile,
  shapeSchema,
  squareSchema,
  userProfileSchema,
  userSchema,
  Address,
  Circle,
  Square,
  userValue,
} from "./common.js";

interface ValidationCase {
  testTitle?: string;
  data: unknown;
  schema: Schema;
  options?: ValidationOptions;
  /** { ok: expected valid set keys } or { error: fragment expected in the message } */
  expected: { ok: string[] } | { error: string };
}

function yieldTestData(): ValidationCase[] {
  // strict user schema: childrenMustBeValid plus an extra Nickname field
  const base = userSchema();
  if (base.shape.kind !== "Class") throw new Error("userSchema must be a class node");
  const strictUserSchema: SchemaNode = {
    ...base,
    childrenMustBeValid: true,
    shape: { kind: "Class", fields: [...base.shape.fields, ["Nickname", scalar("string")]] },
  };

  // address schema with the ZipCode node made non-nilable
  const address = addressSchema();
  if (address.shape.kind !== "Class") throw new Error("addressSchema must be a class node");
  const nonNilableAddressSchema: SchemaNode = {
    ...address,
    shape: {
      kind: "Class",
      fields: address.shape.fields.map(([name, schema]): [string, Schema] => {
        if (name === "ZipCode" && "shape" in schema) {
          return [name, { ...schema, nilable: false }];
        }
        return [name, schema];
      }),
    },
  };

  return [
    {
      testTitle: "validate simple primitive",
      data: "this is a string",
      schema: scalar("string"),
      expected: { ok: [] },
    },
    {
      testTitle: "validate invalid simple primitive",
      data: 42,
      schema: scalar("string"),
      expected: { error: "expected string, found number" },
    },
    {
      testTitle: "validate class instance against its class schema",
      data: sampleUser(),
      schema: userSchema(),
      expected: { ok: [] },
    },
    {
      testTitle: "children_must_be_valid fails when a schema field is absent from data",
      data: sampleUser(),
      schema: strictUserSchema,
      expected: { error: "missing required field 'Nickname'" },
    },
    {
      testTitle: "validate plain object against map[string]any schema",
      data: { a: 1, b: "x" },
      schema: jsonMapSchema(),
      expected: { ok: [] },
    },
    {
      // TS object keys are always strings, so go's "int map key invalid
      // against a json string key schema" case becomes a value-kind mismatch
      testTitle: "map value with the wrong kind fails against the value schema",
      data: { a: 1 },
      schema: mapNode(jsonScalar("string"), jsonScalar("string")),
      expected: { error: "expected string, found number" },
    },
    {
      testTitle: "validate nested item (map + list of any)",
      data: sampleNestedItem(),
      schema: nestedItemSchema(),
      expected: { ok: [] },
    },
    {
      testTitle: "validate list of products",
      data: [sampleProduct(), sampleProduct()],
      schema: listOfProductsSchema(),
      expected: { ok: [] },
    },
    {
      testTitle: "list with a non-string element fails at the element schema",
      data: ["ok", 5],
      schema: sequenceNode(jsonScalar("string")),
      expected: { error: "expected string, found number" },
    },
    {
      testTitle: "validate employee (nested class, string list, string->int map)",
      data: sampleEmployee(),
      schema: employeeSchema(),
      expected: { ok: [] },
    },
    {
      testTitle: "validate company (list of users through an indirection)",
      data: sampleCompany(),
      schema: companySchema(),
      expected: { ok: [] },
    },
    {
      testTitle: "validate user profile with nested address",
      data: sampleUserProfile(),
      schema: userProfileSchema(),
      expected: { ok: [] },
    },
    {
      testTitle: "null zip code is valid against the nilable Option schema",
      data: new Address({ Street: "1 Main St", City: "Springfield", ZipCode: null }),
      schema: addressSchema(),
      expected: { ok: [] },
    },
    {
      testTitle: "null zip code fails against a non-nilable schema",
      data: new Address({ Street: "1 Main St", City: "Springfield", ZipCode: null }),
      schema: nonNilableAddressSchema,
      expected: { error: "data cannot be nil" },
    },
    {
      testTitle: "polymorphic set validates Circle and reports the matched key",
      data: new Circle({ Radius: 1.5 }),
      schema: shapeSchema(),
      expected: { ok: ["Circle"] },
    },
    {
      // without childrenMustBeValid, Square first-matches Circle (unknown
      // fields are skipped, as in go)
      testTitle: "without children_must_be_valid, Square first-matches Circle",
      data: new Square({ Side: 2.0 }),
      schema: shapeSchema(),
      expected: { ok: ["Circle"] },
    },
    {
      testTitle: "with children_must_be_valid, Square matches only the Square node",
      data: new Square({ Side: 2.0 }),
      schema: schemaSet("default", [
        ["Circle", childrenMustBeValid(circleSchema())],
        ["Square", childrenMustBeValid(squareSchema())],
      ]),
      expected: { ok: ["Square"] },
    },
    {
      testTitle: "enum data validates against the enum node via its variant payload",
      data: { variant: "Circle", payload: { Radius: 1.5 } },
      schema: enumNode([
        ["Circle", circleSchema()],
        ["Square", squareSchema()],
      ]),
      expected: { ok: [] },
    },
    {
      testTitle: "scalar is not valid against the shape set",
      data: 42,
      schema: shapeSchema(),
      expected: { error: "expected class, found number" },
    },
    {
      testTitle: "class-instance data is invalid against a map schema (kind mismatch)",
      data: sampleUser(),
      schema: mapUserSchema(),
      expected: { error: "expected map, found class-instance" },
    },
    {
      testTitle: "map string -> user validates against map[int]user schema (key coerced)",
      data: { "1": userValue() },
      schema: mapUserSchema(),
      expected: { ok: [] },
    },
  ];
}

test("validation_cases", () => {
  const report = new TestReport(
    "validation_cases",
    "Data-driven suite for src/application/validation.ts (ADR 0004/0009): validate / validateNode.",
  );
  for (const testCase of yieldTestData()) {
    report.beginCase(newTestTitle(testCase.testTitle));
    try {
      const outcome = validate(testCase.data, testCase.schema, testCase.options ?? {});
      if ("ok" in testCase.expected) {
        if (!eqArrays(outcome.validSetKeys, testCase.expected.ok)) {
          report.failCase(
            `valid set keys: expected ${JSON.stringify(testCase.expected.ok)}, got ${JSON.stringify(outcome.validSetKeys)}`,
          );
        }
      } else {
        report.failCase(`expected error containing ${JSON.stringify(testCase.expected.error)}, got a pass`);
      }
    } catch (err) {
      if ("error" in testCase.expected) {
        const message = err instanceof Error ? err.message : String(err);
        if (!message.includes(testCase.expected.error)) {
          report.failCase(`error: expected fragment ${JSON.stringify(testCase.expected.error)}, got ${JSON.stringify(message)}`);
        }
      } else {
        report.failCase(`expected ok, got error: ${err instanceof Error ? err.message : String(err)}`);
      }
    }
  }

  // custom validator registry case (go's WithCustomValidators, keyed by the
  // data's runtime kind — "number" here)
  const evenValidator: ValidationOptions = {
    customValidators: {
      number: (data) => {
        if ((data as number) % 2 !== 0) {
          throw SchemaError.validationFailed(`${data} is not even`);
        }
      },
    },
  };
  const intSchema = scalar("i64");
  const customCases: [number, boolean, string][] = [
    [4, true, "custom validator accepts even number"],
    [3, false, "custom validator rejects odd number"],
  ];
  for (const [value, expectedOk, title] of customCases) {
    report.beginCase(newTestTitle(`custom: ${title}`));
    let ok = false;
    let message = "";
    try {
      validate(value, intSchema, evenValidator);
      ok = true;
    } catch (err) {
      message = err instanceof Error ? err.message : String(err);
    }
    if (ok !== expectedOk) {
      report.failCase(`expected ok=${expectedOk}, got ok=${ok} ${message}`);
    }
  }

  report.finish();
});

function eqArrays(a: string[], b: string[]): boolean {
  return a.length === b.length && a.every((value, index) => value === b[index]);
}
