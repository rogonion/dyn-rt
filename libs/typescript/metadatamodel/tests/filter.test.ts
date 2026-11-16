// Data-driven suite for the metadatamodel `filter` subdomain
// (ADR 0004/0009): src/filter/{domain,application,conditions}.ts —
// evaluating query conditions (logical operators + per-field conditions)
// against a root sequence in source data, returning the excluded indexes
// (port of go-metadatamodel/filter's filter_data_test.go).
// Port of libs/rust/metadatamodel/tests/filter/main.rs.

import { test } from "vitest";
import { TestReport, newTestTitle } from "@dyn-rt/test-support";
import { DynObject } from "@dyn-rt/manipulation";
import { object, type Value } from "@dyn-rt/shared";
import { FIELDS_KEY, props, type MetadataModel } from "../src/core/index.js";
import { DataFilter } from "../src/filter/index.js";
import {
  condition,
  conditionProps,
  fieldType,
  query,
} from "../src/filter/domain.js";

// ---------------------------------------------------------------------------
// source data shapes (go's testdata.Product / testdata.UserProfile with
// array fields, since model cardinality defaults to array)
// ---------------------------------------------------------------------------

class ProductRow {
  ID: number[] = [];
  Name: string[] = [];
  Price: number[] = [];
  Created: string[] = [];
}

class AddressRow {
  Street: string[] = [];
  City: string[] = [];
}

class UserProfileRow {
  Name: string[] = [];
  Age: number[] = [];
  Address: AddressRow[] = [];
}

// ---------------------------------------------------------------------------
// sample models (section:key props, fields-as-arrays, [*] templates)
// ---------------------------------------------------------------------------

function fld(pathKey: string, name: string, dataType: string): Value {
  return [
    {
      [props.GROUP_PATH_KEY]: pathKey,
      [props.GROUP_NAME]: name,
      [props.FIELD_DATA_TYPE]: dataType,
    },
  ];
}

function grp(pathKey: string, name: string, fields: [string, Value][]): Value {
  return [
    {
      [props.GROUP_PATH_KEY]: pathKey,
      [props.GROUP_NAME]: name,
      [FIELDS_KEY]: [object(fields)],
    },
  ];
}

/** Product model with fields ID, Name, Price, Created. */
function productModel(): MetadataModel {
  return {
    [props.GROUP_PATH_KEY]: "$",
    [props.GROUP_NAME]: "Product",
    [FIELDS_KEY]: [
      {
        ID: fld("$['group:fields'][*].ID[*]", "ID", "Number"),
        Name: fld("$['group:fields'][*].Name[*]", "Name", "Text"),
        Price: fld("$['group:fields'][*].Price[*]", "Price", "Number"),
        Created: fld("$['group:fields'][*].Created[*]", "Created", "Timestamp"),
      },
    ],
  };
}

/** UserProfile model with fields Name, Age and nested group Address
 * (Street, City). */
function userProfileModel(): MetadataModel {
  return {
    [props.GROUP_PATH_KEY]: "$",
    [props.GROUP_NAME]: "UserProfile",
    [FIELDS_KEY]: [
      {
        Name: fld("$['group:fields'][*].Name[*]", "Name", "Text"),
        Age: fld("$['group:fields'][*].Age[*]", "Age", "Number"),
        Address: grp("$['group:fields'][*].Address[*]", "Address", [
          [
            "Street",
            fld(
              "$['group:fields'][*].Address[*]['group:fields'][*].Street[*]",
              "Street",
              "Text",
            ),
          ],
          [
            "City",
            fld(
              "$['group:fields'][*].Address[*]['group:fields'][*].City[*]",
              "City",
              "Text",
            ),
          ],
        ]),
      },
    ],
  };
}

// ---------------------------------------------------------------------------
// query condition builders
// ---------------------------------------------------------------------------

function qLogical(operator: string, negate: boolean, conditions: Value[]): Value {
  return {
    [query.TYPE]: query.TYPE_LOGICAL_OPERATOR,
    [query.LOGICAL_OPERATOR]: operator,
    [query.NEGATE]: negate,
    [query.VALUE]: conditions,
  };
}

function qFieldGroup(negate: boolean, entries: [string, Value][]): Value {
  return {
    [query.TYPE]: query.TYPE_FIELD_GROUP,
    [query.NEGATE]: negate,
    [query.VALUE]: object(entries),
  };
}

function conditions(entries: [string, Value][]): Value {
  return object(entries);
}

function numberCondValue(value: number): Value {
  return {
    [conditionProps.ASSUMED_FIELD_TYPE]: fieldType.NUMBER,
    [conditionProps.VALUE]: value,
  };
}

function textCond(name: string, value: string): Value {
  return conditions([
    [
      name,
      {
        [conditionProps.ASSUMED_FIELD_TYPE]: fieldType.TEXT,
        [conditionProps.VALUE]: value,
      },
    ],
  ]);
}

// ---------------------------------------------------------------------------
// source data
// ---------------------------------------------------------------------------

function products(): unknown {
  return [
    Object.assign(new ProductRow(), {
      ID: [0],
      Name: ["Product 0"],
      Price: [11.0],
      Created: ["2023-05-01T10:00:00Z"],
    }),
    Object.assign(new ProductRow(), {
      ID: [1],
      Name: ["Product 1"],
      Created: ["2024-06-01T10:00:00Z"],
    }),
    Object.assign(new ProductRow(), {
      ID: [2],
      Name: ["Product 2"],
      Created: ["2025-01-15T08:30:00Z"],
    }),
    Object.assign(new ProductRow(), {
      ID: [3],
      Name: ["Product 3"],
      Created: ["2022-12-31T23:59:59Z"],
    }),
  ];
}

function userProfiles(): unknown {
  return [
    Object.assign(new UserProfileRow(), {
      Name: ["User 0"],
      Age: [10],
      Address: [
        Object.assign(new AddressRow(), { Street: ["Street 1"], City: ["City 1"] }),
      ],
    }),
    Object.assign(new UserProfileRow(), {
      Name: ["User 2"],
      Age: [20],
      Address: [
        Object.assign(new AddressRow(), { Street: ["Street 2"], City: ["City 2"] }),
      ],
    }),
    Object.assign(new UserProfileRow(), {
      Name: ["User 3"],
      Age: [30],
      Address: [
        Object.assign(new AddressRow(), { Street: ["Street 3"], City: ["City 3"] }),
        Object.assign(new AddressRow(), { Street: ["Street 4"], City: ["City 4"] }),
      ],
    }),
  ];
}

// ---------------------------------------------------------------------------
// cases (port of go's filterDataTestData)
// ---------------------------------------------------------------------------

interface FilterCase {
  testTitle?: string;
  source: () => unknown;
  model: () => MetadataModel;
  query: Value;
  rootJsonPathKey?: string;
  rootJsonPathToValue?: string;
  expectedExclude: number[];
  expectError?: string;
  silenceErrors?: boolean;
}

function yieldTestData(): FilterCase[] {
  const idKey = "$['group:fields'][*].ID[*]";
  const nameKey = "$['group:fields'][*].Name[*]";
  const createdKey = "$['group:fields'][*].Created[*]";
  const addressKey = "$['group:fields'][*].Address[*]";
  const cityKey = "$['group:fields'][*].Address[*]['group:fields'][*].City[*]";

  return [
    // go case 1: Product model, AND(ID > 0 AND ID < 3, Name EndsWith "2")
    {
      source: products,
      model: productModel,
      query: qLogical(query.AND, false, [
        qFieldGroup(false, [
          [
            idKey,
            conditions([
              [condition.GREATER_THAN, numberCondValue(0)],
              [condition.LESS_THAN, numberCondValue(3)],
            ]),
          ],
        ]),
        qFieldGroup(false, [[nameKey, textCond(condition.ENDS_WITH, "2")]]),
      ]),
      expectedExclude: [0, 1, 3],
      testTitle:
        "Product model: AND(ID>0 AND ID<3, Name EndsWith 2) excludes [0,1,3] (go case 1)",
    },
    // go case 2: UserProfile model, OR(Address NoOfEntriesGreaterThan 1, Address.City BeginsWith "Streete")
    {
      source: userProfiles,
      model: userProfileModel,
      query: qLogical(query.OR, false, [
        qFieldGroup(false, [
          [
            addressKey,
            conditions([
              [
                condition.NO_OF_ENTRIES_GREATER_THAN,
                { [conditionProps.VALUE]: 1 },
              ],
            ]),
          ],
        ]),
        qFieldGroup(false, [[cityKey, textCond(condition.BEGINS_WITH, "Streete")]]),
      ]),
      expectedExclude: [0, 1, 2],
      testTitle:
        "UserProfile model: OR(Address NoOfEntriesGreaterThan 1, City BeginsWith 'Streete') excludes all (go case 2)",
    },
    // go case 3: focus on Address of profile at index 2, negated BeginsWith "City 4"
    {
      source: userProfiles,
      model: userProfileModel,
      query: qFieldGroup(true, [[cityKey, textCond(condition.BEGINS_WITH, "City 4")]]),
      rootJsonPathKey: "$['group:fields'][*].Address[*]",
      rootJsonPathToValue: "$[2].Address",
      expectedExclude: [1],
      testTitle:
        "UserProfile focus on $[2].Address, negated City BeginsWith 'City 4' excludes [1] (go case 3)",
    },
    // timestamp condition: Created GreaterThan 2024-01-01 (yyyy-mm-dd)
    {
      source: products,
      model: productModel,
      query: qFieldGroup(false, [
        [
          createdKey,
          conditions([
            [
              condition.GREATER_THAN,
              {
                [conditionProps.ASSUMED_FIELD_TYPE]: fieldType.TIMESTAMP,
                [conditionProps.DATE_TIME_FORMAT]: "yyyy-mm-dd",
                [conditionProps.VALUE]: "2024-01-01T00:00:00Z",
              },
            ],
          ]),
        ],
      ]),
      expectedExclude: [0, 3],
      testTitle: "timestamp condition: Created > 2024-01-01 (yyyy-mm-dd) excludes [0,3]",
    },
    // text condition with Values array + CaseInsensitive
    {
      source: products,
      model: productModel,
      query: qFieldGroup(false, [
        [
          nameKey,
          conditions([
            [
              condition.EQUAL_TO,
              {
                [conditionProps.ASSUMED_FIELD_TYPE]: fieldType.TEXT,
                [conditionProps.CASE_INSENSITIVE]: true,
                [conditionProps.VALUES]: ["product 1", "PRODUCT 3"],
              },
            ],
          ]),
        ],
      ]),
      expectedExclude: [0, 2],
      testTitle: "text EqualTo with Values array, case-insensitive, excludes [0,2]",
    },
    // unknown query condition type is an error (or false when silenced)
    {
      source: products,
      model: productModel,
      query: { [query.TYPE]: "Nope" },
      expectedExclude: [],
      expectError: "unknown query condition type",
      testTitle: "unknown query condition type errors",
    },
    {
      source: products,
      model: productModel,
      query: { [query.TYPE]: "Nope" },
      expectedExclude: [0, 1, 2, 3],
      silenceErrors: true,
      testTitle: "silenced errors make the condition false, excluding all",
    },
  ];
}

test("filter_data_cases", () => {
  const report = new TestReport(
    "filter_data_cases",
    "filter DataFilter query-condition evaluation (src/filter)",
  );
  for (const testCase of yieldTestData()) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const source = testCase.source();
    const model = testCase.model();
    const dataFilter = new DataFilter(DynObject.new(source), model);
    dataFilter.silenceErrors = testCase.silenceErrors ?? false;
    try {
      const exclude = dataFilter.filter(
        testCase.query,
        testCase.rootJsonPathKey ?? "",
        testCase.rootJsonPathToValue ?? "",
      );
      if (testCase.expectError !== undefined) {
        report.failCase(
          `expected error containing ${JSON.stringify(testCase.expectError)}, got Ok(${JSON.stringify(exclude)})`,
        );
      } else if (JSON.stringify(exclude) !== JSON.stringify(testCase.expectedExclude)) {
        report.failCase(
          `exclude indexes: expected ${JSON.stringify(testCase.expectedExclude)}, got ${JSON.stringify(exclude)}`,
        );
      }
    } catch (err) {
      if (testCase.expectError !== undefined) {
        const message = err instanceof Error ? err.message : String(err);
        if (!message.includes(testCase.expectError)) {
          report.failCase(
            `error expected to contain ${JSON.stringify(testCase.expectError)}, got ${message}`,
          );
        }
      } else {
        report.failCase(`unexpected error: ${err instanceof Error ? err.message : String(err)}`);
      }
    }
  }
  report.finish();
});
