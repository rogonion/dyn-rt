// Data-driven suites for the metadatamodel `database` subdomain
// (ADR 0004/0009): src/database/{domain,application}.ts — extraction of a
// table collection's columns (`getColumnFields`, port of go's
// GetColumnFields) and get/set/delete of field values in source data
// through those columns (`FieldValue`, port of go's FieldValue).
// Port of libs/rust/metadatamodel/tests/database/main.rs.

import { test } from "vitest";
import { TestReport, newTestTitle } from "@dyn-rt/test-support";
import { areEqual, DynObject } from "@dyn-rt/manipulation";
import { classNode, scalar, sequenceNode, type Schema, type SchemaNode } from "@dyn-rt/schema";
import { object, type JsonObject, type Value } from "@dyn-rt/shared";
import { FIELDS_KEY, props, type MetadataModel } from "../src/core/index.js";
import { PropMatch } from "../src/fieldcolumns/index.js";
import {
  FieldValue,
  getColumnFields,
  type GetColumnFieldsOptions,
} from "../src/database/index.js";

/** Renders a value for failure messages. */
function render(value: unknown): string {
  try {
    return JSON.stringify(value) ?? String(value);
  } catch {
    return String(value);
  }
}

// ---------------------------------------------------------------------------
// sample models (port of go-metadatamodel/testdata Product/Employee models,
// with the section:key property names and fields-as-arrays)
// ---------------------------------------------------------------------------

/** A field entry: array whose first element is the props object, carrying
 * the db:* properties like go's testdata models. */
function dbField(
  pathKey: string,
  name: string,
  column: string,
  uid: string,
  depth: number,
  table: string,
  primary: boolean,
): Value {
  const entries: JsonObject = {
    [props.GROUP_PATH_KEY]: pathKey,
    [props.GROUP_NAME]: name,
    [props.DB_TABLE_COLLECTION_UID]: uid,
    [props.DB_JOIN_DEPTH]: depth,
    [props.DB_TABLE_COLLECTION_NAME]: table,
    [props.DB_COLUMN_NAME]: column,
  };
  if (primary) {
    entries[props.GROUP_IS_PRIMARY_KEY] = true;
  }
  return [entries];
}

/** A group entry (array with props object) holding nested fields. */
function dbGroup(
  pathKey: string,
  name: string,
  uid: string,
  depth: number,
  table: string,
  fields: [string, Value][],
): Value {
  return [
    {
      [props.GROUP_PATH_KEY]: pathKey,
      [props.GROUP_NAME]: name,
      [props.DB_TABLE_COLLECTION_UID]: uid,
      [props.DB_JOIN_DEPTH]: depth,
      [props.DB_TABLE_COLLECTION_NAME]: table,
      [FIELDS_KEY]: [object(fields)],
    },
  ];
}

/** Root group "Product" (uid Product, join depth 0) with fields ID, Name,
 * Price — port of go's testdata.ProductMetadataModel. */
function productModel(): MetadataModel {
  return {
    [props.GROUP_PATH_KEY]: "$",
    [props.GROUP_NAME]: "Product",
    [props.DB_TABLE_COLLECTION_UID]: "Product",
    [props.DB_JOIN_DEPTH]: 0,
    [props.DB_TABLE_COLLECTION_NAME]: "Product",
    [FIELDS_KEY]: [
      {
        ID: dbField("$['group:fields'][*].ID[*]", "ID", "ID", "Product", 0, "Product", true),
        Name: dbField("$['group:fields'][*].Name[*]", "Name", "Name", "Product", 0, "Product", false),
        Price: dbField(
          "$['group:fields'][*].Price[*]",
          "Price",
          "Price",
          "Product",
          0,
          "Product",
          false,
        ),
      },
    ],
  };
}

/** Root group "Employee" (uid Employee, join depth 0) with field ID and a
 * nested group "Profile" (uid Profile, join depth 1) holding Age and Bio —
 * port of go's testdata.EmployeeMetadataModel. */
function employeeModel(): MetadataModel {
  return {
    [props.GROUP_PATH_KEY]: "$",
    [props.GROUP_NAME]: "Employee",
    [props.DB_TABLE_COLLECTION_UID]: "Employee",
    [props.DB_JOIN_DEPTH]: 0,
    [props.DB_TABLE_COLLECTION_NAME]: "Employee",
    [FIELDS_KEY]: [
      {
        ID: dbField("$['group:fields'][*].ID[*]", "ID", "ID", "Employee", 0, "Employee", true),
        Profile: dbGroup(
          "$['group:fields'][*].Profile[*]",
          "Profile",
          "Profile",
          1,
          "Profile",
          [
            [
              "Age",
              dbField(
                "$['group:fields'][*].Profile[*]['group:fields'][*].Age[*]",
                "Age",
                "Age",
                "Profile",
                1,
                "Profile",
                false,
              ),
            ],
            [
              "Bio",
              dbField(
                "$['group:fields'][*].Profile[*]['group:fields'][*].Bio[*]",
                "Bio",
                "Bio",
                "Profile",
                1,
                "Profile",
                false,
              ),
            ],
          ],
        ),
      },
    ],
  };
}

/** Small model whose field has no db:column_name. */
function noColumnNameModel(): MetadataModel {
  return {
    [props.GROUP_PATH_KEY]: "$",
    [props.GROUP_NAME]: "Broken",
    [props.DB_TABLE_COLLECTION_UID]: "Broken",
    [FIELDS_KEY]: [
      {
        A: [
          {
            [props.GROUP_PATH_KEY]: "$['group:fields'][*].A[*]",
            [props.GROUP_NAME]: "A",
            [props.DB_TABLE_COLLECTION_UID]: "Broken",
          },
        ],
      },
    ],
  };
}

/** Small model with two fields sharing one db:column_name. */
function duplicateColumnModel(): MetadataModel {
  return {
    [props.GROUP_PATH_KEY]: "$",
    [props.GROUP_NAME]: "Dup",
    [props.DB_TABLE_COLLECTION_UID]: "Dup",
    [FIELDS_KEY]: [
      {
        A: dbField("$['group:fields'][*].A[*]", "A", "Dup", "Dup", 0, "Dup", false),
        B: dbField("$['group:fields'][*].B[*]", "B", "Dup", "Dup", 0, "Dup", false),
      },
    ],
  };
}

// ---------------------------------------------------------------------------
// getColumnFields cases
// ---------------------------------------------------------------------------

interface GetColumnFieldsCase {
  testTitle?: string;
  model: () => MetadataModel;
  options: GetColumnFieldsOptions;
  /** expected column names in read order, each with the field's path_key */
  expected?: [string, string][];
  expectError?: string;
}

function uidOptions(uid: string): GetColumnFieldsOptions {
  return { tableCollectionUid: uid };
}

function depthNameOptions(depth: number, name: string): GetColumnFieldsOptions {
  return { joinDepth: depth, tableCollectionName: name };
}

function yieldGetColumnFieldsData(): GetColumnFieldsCase[] {
  return [
    {
      model: productModel,
      options: uidOptions("Product"),
      expected: [
        ["ID", "$['group:fields'][*].ID[*]"],
        ["Name", "$['group:fields'][*].Name[*]"],
        ["Price", "$['group:fields'][*].Price[*]"],
      ],
      testTitle: "Product model by table_collection_uid",
    },
    {
      model: productModel,
      options: depthNameOptions(0, "Product"),
      expected: [
        ["ID", "$['group:fields'][*].ID[*]"],
        ["Name", "$['group:fields'][*].Name[*]"],
        ["Price", "$['group:fields'][*].Price[*]"],
      ],
      testTitle: "Product model by join_depth 0 + table_collection_name",
    },
    {
      model: employeeModel,
      options: uidOptions("Employee"),
      expected: [["ID", "$['group:fields'][*].ID[*]"]],
      testTitle: "Employee model by uid Employee collects only root fields (groups not collected)",
    },
    {
      model: employeeModel,
      options: uidOptions("Profile"),
      expected: [
        ["Age", "$['group:fields'][*].Profile[*]['group:fields'][*].Age[*]"],
        ["Bio", "$['group:fields'][*].Profile[*]['group:fields'][*].Bio[*]"],
      ],
      testTitle: "Employee model by uid Profile collects nested group fields",
    },
    {
      model: employeeModel,
      options: depthNameOptions(1, "Profile"),
      expected: [
        ["Age", "$['group:fields'][*].Profile[*]['group:fields'][*].Age[*]"],
        ["Bio", "$['group:fields'][*].Profile[*]['group:fields'][*].Bio[*]"],
      ],
      testTitle: "Employee model by join_depth 1 + name Profile",
    },
    {
      model: employeeModel,
      options: {
        tableCollectionUid: "Profile",
        add: new PropMatch([[props.DB_COLUMN_NAME, "Age"]]),
      },
      expected: [["Age", "$['group:fields'][*].Profile[*]['group:fields'][*].Age[*]"]],
      testTitle: "Employee model uid Profile with add db:column_name=Age collects only Age",
    },
    {
      model: employeeModel,
      options: {
        tableCollectionUid: "Profile",
        skip: new PropMatch([[props.GROUP_NAME, "Bio"]]),
      },
      expected: [["Age", "$['group:fields'][*].Profile[*]['group:fields'][*].Age[*]"]],
      testTitle: "Employee model uid Profile with skip group:name=Bio skips Bio",
    },
    {
      model: productModel,
      options: {},
      expectError: "is required",
      testTitle: "no uid and no join_depth+name is an error",
    },
    {
      model: noColumnNameModel,
      options: uidOptions("Broken"),
      expectError: "db:column_name not found",
      testTitle: "field without db:column_name is an error",
    },
    {
      model: duplicateColumnModel,
      options: uidOptions("Dup"),
      expectError: "duplicate db:column_name",
      testTitle: "duplicate db:column_name is an error",
    },
  ];
}

test("get_column_fields_cases", () => {
  const report = new TestReport(
    "get_column_fields_cases",
    "database getColumnFields (src/database/application.ts)",
  );
  for (const testCase of yieldGetColumnFieldsData()) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const model = testCase.model();
    try {
      const fields = getColumnFields(model, testCase.options);
      if (testCase.expected === undefined) {
        report.failCase("expected an error but got Ok");
        continue;
      }
      const gotOrder = fields.readOrder;
      const expectedOrder = testCase.expected.map(([name]) => name);
      if (JSON.stringify(gotOrder) !== JSON.stringify(expectedOrder)) {
        report.failCase(
          `read_order: expected ${JSON.stringify(expectedOrder)}, got ${JSON.stringify(gotOrder)}`,
        );
      }
      for (const [name, pathKey] of testCase.expected) {
        const propsValue = fields.get(name);
        if (propsValue === undefined) {
          report.failCase(`column ${name}: missing`);
          continue;
        }
        const gotKey =
          typeof propsValue === "object" && propsValue !== null && !Array.isArray(propsValue)
            ? (propsValue as JsonObject)[props.GROUP_PATH_KEY]
            : undefined;
        if (gotKey !== pathKey) {
          report.failCase(`column ${name}: path_key expected ${JSON.stringify(pathKey)}, got ${JSON.stringify(gotKey)}`);
        }
      }
    } catch (err) {
      if (testCase.expectError !== undefined) {
        const message = err instanceof Error ? err.message : String(err);
        if (!message.includes(testCase.expectError)) {
          report.failCase(`error expected to contain ${JSON.stringify(testCase.expectError)}, got ${message}`);
        }
      } else {
        report.failCase(`unexpected error: ${err instanceof Error ? err.message : String(err)}`);
      }
    }
  }
  report.finish();
});

// ---------------------------------------------------------------------------
// field value cases (port of go's field_value_test.go, on native classes
// with array fields like go's testdata.Product/Employee)
// ---------------------------------------------------------------------------

class ProductData {
  ID: number[] = [];
  Name: string[] = [];
  Price: number[] = [];
}

class ProfileData {
  Age: number[] = [];
  Bio: string[] = [];
}

class EmployeeData {
  ID: number[] = [];
  Profile: ProfileData[] = [];
}

function intSeqNode(): SchemaNode {
  return sequenceNode(scalar("i64"));
}

function stringSeqNode(): SchemaNode {
  return sequenceNode(scalar("string"));
}

function floatSeqNode(): SchemaNode {
  return sequenceNode(scalar("f64"));
}

function productDataSchema(): SchemaNode {
  return classNode("ProductData", [
    ["ID", intSeqNode()],
    ["Name", stringSeqNode()],
    ["Price", floatSeqNode()],
  ]);
}

function profileDataSchema(): SchemaNode {
  return classNode("ProfileData", [
    ["Age", intSeqNode()],
    ["Bio", stringSeqNode()],
  ]);
}

function employeeDataSchema(): SchemaNode {
  return classNode("EmployeeData", [
    ["ID", intSeqNode()],
    ["Profile", sequenceNode(profileDataSchema())],
  ]);
}

// source factories (each case gets a fresh source to mutate)
function productWithId(): unknown {
  return Object.assign(new ProductData(), { ID: [1] });
}

function productWithPrice(): unknown {
  return Object.assign(new ProductData(), { ID: [1], Price: [0.0] });
}

function productWithTwinkies(): unknown {
  return Object.assign(new ProductData(), { ID: [1], Name: ["Twinkies"] });
}

function productPriceDeleted(): unknown {
  return Object.assign(new ProductData(), { ID: [1] });
}

function employeeWithEmptyProfile(): unknown {
  return Object.assign(new EmployeeData(), { ID: [1], Profile: [new ProfileData()] });
}

function employeeWithAge16(): unknown {
  return Object.assign(new EmployeeData(), {
    ID: [1],
    Profile: [Object.assign(new ProfileData(), { Age: [16] })],
  });
}

function productsVecWithId7(): unknown {
  return [Object.assign(new ProductData(), { ID: [7] })];
}

function nameMetaValue(): unknown {
  return { Name: [{ x: { y: 5 } }] };
}

type FieldValueOp =
  | {
      kind: "get";
      column: string;
      suffix: string;
      indices: number[];
      expectedCount: number;
      expected: unknown;
    }
  | {
      kind: "set";
      column: string;
      value: Value;
      suffix: string;
      indices: number[];
      expectedCount: number;
      expectedSource: () => unknown;
    }
  | {
      kind: "delete";
      column: string;
      suffix: string;
      indices: number[];
      expectedCount: number;
      expectedSource: () => unknown;
    }
  | { kind: "getUnknownColumn"; column: string };

interface FieldValueCase {
  testTitle?: string;
  /** the metadata model to extract the columns from */
  model: () => MetadataModel;
  options: GetColumnFieldsOptions;
  /** fresh source data to manipulate */
  source: () => unknown;
  schema?: Schema;
  op: FieldValueOp;
}

function yieldFieldValueData(): FieldValueCase[] {
  return [
    // go TestDatabase_FieldValueOnProduct: Get ID
    {
      model: productModel,
      options: depthNameOptions(0, "Product"),
      source: productWithId,
      schema: productDataSchema(),
      op: {
        kind: "get",
        column: "ID",
        suffix: "",
        indices: [],
        expectedCount: 1,
        expected: 1,
      },
      testTitle: "get ID on Product returns the stored value (go FieldValueOnProduct Get)",
    },
    // go TestDatabase_FieldValueOnProduct: Set Name
    {
      model: productModel,
      options: depthNameOptions(0, "Product"),
      source: productWithId,
      schema: productDataSchema(),
      op: {
        kind: "set",
        column: "Name",
        value: "Twinkies",
        suffix: "",
        indices: [],
        expectedCount: 1,
        expectedSource: productWithTwinkies,
      },
      testTitle: "set Name on Product inserts the value (go FieldValueOnProduct Set)",
    },
    // go TestDatabase_FieldValueOnProduct: Delete Price
    {
      model: productModel,
      options: depthNameOptions(0, "Product"),
      source: productWithPrice,
      schema: productDataSchema(),
      op: {
        kind: "delete",
        column: "Price",
        suffix: "",
        indices: [],
        expectedCount: 1,
        expectedSource: productPriceDeleted,
      },
      testTitle: "delete Price on Product empties the value (go FieldValueOnProduct Delete)",
    },
    // go TestDatabase_FieldValueOnEmployee: Set Age on the nested Profile
    {
      model: employeeModel,
      options: depthNameOptions(1, "Profile"),
      source: employeeWithEmptyProfile,
      schema: employeeDataSchema(),
      op: {
        kind: "set",
        column: "Age",
        value: 16,
        suffix: "",
        indices: [],
        expectedCount: 1,
        expectedSource: employeeWithAge16,
      },
      testTitle: "set Age on Employee's nested Profile (go FieldValueOnEmployee Set)",
    },
    // source root is a sequence: first index addresses the root entry
    {
      model: productModel,
      options: uidOptions("Product"),
      source: productsVecWithId7,
      schema: sequenceNode(productDataSchema()),
      op: {
        kind: "get",
        column: "ID",
        suffix: "",
        indices: [0],
        expectedCount: 1,
        expected: 7,
      },
      testTitle: "get ID on a Product[] source addresses the root entry first",
    },
    // suffix path appended to the field's data path
    {
      model: productModel,
      options: uidOptions("Product"),
      source: nameMetaValue,
      op: {
        kind: "get",
        column: "Name",
        suffix: "x.y",
        indices: [],
        expectedCount: 1,
        expected: 5,
      },
      testTitle: "get with a suffix path appends to the field's data path",
    },
    {
      model: productModel,
      options: uidOptions("Product"),
      source: productWithId,
      op: { kind: "getUnknownColumn", column: "Nope" },
      testTitle: "get with an unknown column name is an error",
    },
  ];
}

test("field_value_cases", () => {
  const report = new TestReport(
    "field_value_cases",
    "database FieldValue get/set/delete (src/database/application.ts)",
  );
  for (const testCase of yieldFieldValueData()) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const model = testCase.model();
    let columnFields;
    try {
      columnFields = getColumnFields(model, testCase.options);
    } catch (err) {
      report.failCase(`getColumnFields failed: ${err instanceof Error ? err.message : String(err)}`);
      continue;
    }
    const source = testCase.source();
    const fieldValue = new FieldValue(DynObject.new(source), columnFields);
    const op = testCase.op;
    try {
      if (op.kind === "get") {
        const found = fieldValue.get(op.column, op.suffix, op.indices);
        if (found.length !== op.expectedCount) {
          report.failCase(`count: expected ${op.expectedCount}, got ${found.length}`);
        } else if (found.length > 0 && !areEqual(found[0], op.expected)) {
          report.failCase(`value: expected ${render(op.expected)}, got ${render(found[0])}`);
        }
      } else if (op.kind === "set") {
        const count = fieldValue.set(op.column, op.value, op.suffix, op.indices, testCase.schema);
        if (count !== op.expectedCount) {
          report.failCase(`count: expected ${op.expectedCount}, got ${count}`);
        }
      } else if (op.kind === "delete") {
        const count = fieldValue.delete(op.column, op.suffix, op.indices);
        if (count !== op.expectedCount) {
          report.failCase(`count: expected ${op.expectedCount}, got ${count}`);
        }
      } else {
        // getUnknownColumn: expect an error containing "not found"
        try {
          fieldValue.get(op.column, "", []);
          report.failCase("expected an error but got Ok");
        } catch (err) {
          const message = err instanceof Error ? err.message : String(err);
          if (!message.includes("not found")) {
            report.failCase(`unexpected error: ${message}`);
          }
        }
      }
    } catch (err) {
      report.failCase(`${op.kind} failed: ${err instanceof Error ? err.message : String(err)}`);
    }
    // for set/delete, compare the mutated source with the expected source
    if (op.kind === "set" || op.kind === "delete") {
      const expected = op.expectedSource();
      if (!areEqual(fieldValue.obj.getSource(), expected)) {
        report.failCase(
          `source: expected ${render(expected)}, got ${render(fieldValue.obj.getSource())}`,
        );
      }
    }
  }
  report.finish();
});
