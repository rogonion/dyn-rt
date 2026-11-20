// Data-driven suites for the metadatamodel `flattener` subdomain
// (ADR 0004/0009): src/flattener/{domain,application}.ts — converting
// nested objects/collections into a 2D table (port of
// go-metadatamodel/flattener's flatten_test.go), incl. single-cardinality
// fields/groups and writeToDestination.
// Port of libs/rust/metadatamodel/tests/flattener/main.rs.

import { test } from "vitest";
import { TestReport, newTestTitle } from "@dyn-rt/test-support";
import { areEqual, DynObject } from "@dyn-rt/manipulation";
import { object, type Value } from "@dyn-rt/shared";
import { FIELDS_KEY, props, type MetadataModel } from "../src/core/index.js";
import { Flattener, type FlattenedTable } from "../src/flattener/index.js";

// ---------------------------------------------------------------------------
// source data shapes (go's testdata.User/Product/Employee with array fields)
// ---------------------------------------------------------------------------

class UserRow {
  ID: number[] = [];
  Name: string[] = [];
  Email: string[] = [];
}

class ProductRow {
  ID: number[] = [];
  Name: string[] = [];
  Price: number[] = [];
}

class AddressRow {
  Street: string[] = [];
  City: string[] = [];
  ZipCode: string[] = [];
}

class ProfileRow {
  Name: string[] = [];
  Age: number[] = [];
  Address: AddressRow[] = [];
}

class EmployeeRow {
  ID: number[] = [];
  Skills: string[] = [];
  Profile: ProfileRow[] = [];
}

/** Employee whose Profile group is a single object (group:cardinality single). */
class EmployeeSingleProfile {
  ID: number[] = [];
  Profile: ProfileRow = new ProfileRow();
}

/** User with a single-cardinality scalar field Bio. */
class UserWithBio {
  ID: number[] = [];
  Bio = false;
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

function fldPrimary(pathKey: string, name: string, dataType: string): Value {
  return [
    {
      [props.GROUP_PATH_KEY]: pathKey,
      [props.GROUP_NAME]: name,
      [props.FIELD_DATA_TYPE]: dataType,
      [props.GROUP_IS_PRIMARY_KEY]: true,
    },
  ];
}

function fldSingle(pathKey: string, name: string, dataType: string): Value {
  return [
    {
      [props.GROUP_PATH_KEY]: pathKey,
      [props.GROUP_NAME]: name,
      [props.FIELD_DATA_TYPE]: dataType,
      [props.FIELD_CARDINALITY]: "single",
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

function grpSingle(pathKey: string, name: string, fields: [string, Value][]): Value {
  return [
    {
      [props.GROUP_PATH_KEY]: pathKey,
      [props.GROUP_NAME]: name,
      [props.GROUP_CARDINALITY]: "single",
      [FIELDS_KEY]: [object(fields)],
    },
  ];
}

function userModel(): MetadataModel {
  return {
    [props.GROUP_PATH_KEY]: "$",
    [props.GROUP_NAME]: "User",
    [FIELDS_KEY]: [
      {
        ID: fldPrimary("$['group:fields'][*].ID[*]", "ID", "Number"),
        Name: fld("$['group:fields'][*].Name[*]", "Name", "Text"),
        Email: fld("$['group:fields'][*].Email[*]", "Email", "Text"),
      },
    ],
  };
}

function productModel(): MetadataModel {
  return {
    [props.GROUP_PATH_KEY]: "$",
    [props.GROUP_NAME]: "Product",
    [FIELDS_KEY]: [
      {
        ID: fldPrimary("$['group:fields'][*].ID[*]", "ID", "Number"),
        Name: fld("$['group:fields'][*].Name[*]", "Name", "Text"),
        Price: fld("$['group:fields'][*].Price[*]", "Price", "Number"),
      },
    ],
  };
}

function employeeModel(): MetadataModel {
  return {
    [props.GROUP_PATH_KEY]: "$",
    [props.GROUP_NAME]: "Employee",
    [FIELDS_KEY]: [
      {
        ID: fldPrimary("$['group:fields'][*].ID[*]", "ID", "Number"),
        Profile: grp("$['group:fields'][*].Profile[*]", "Profile", [
          ["Name", fld("$['group:fields'][*].Profile[*]['group:fields'][*].Name[*]", "Name", "Text")],
          ["Age", fld("$['group:fields'][*].Profile[*]['group:fields'][*].Age[*]", "Age", "Number")],
          [
            "Address",
            grp("$['group:fields'][*].Profile[*]['group:fields'][*].Address[*]", "Address", [
              ["Street", fld("$['group:fields'][*].Profile[*]['group:fields'][*].Address[*]['group:fields'][*].Street[*]", "Street", "Text")],
              ["City", fld("$['group:fields'][*].Profile[*]['group:fields'][*].Address[*]['group:fields'][*].City[*]", "City", "Text")],
              ["ZipCode", fld("$['group:fields'][*].Profile[*]['group:fields'][*].Address[*]['group:fields'][*].ZipCode[*]", "ZipCode", "Text")],
            ]),
          ],
        ]),
        Skills: fld("$['group:fields'][*].Skills[*]", "Skills", "Text"),
      },
    ],
  };
}

/** User model with a single-cardinality Bio field. */
function userBioModel(): MetadataModel {
  return {
    [props.GROUP_PATH_KEY]: "$",
    [props.GROUP_NAME]: "User",
    [FIELDS_KEY]: [
      {
        ID: fldPrimary("$['group:fields'][*].ID[*]", "ID", "Number"),
        Bio: fldSingle("$['group:fields'][*].Bio[*]", "Bio", "Boolean"),
      },
    ],
  };
}

/** Employee model with a single-cardinality Profile group. */
function employeeSingleProfileModel(): MetadataModel {
  return {
    [props.GROUP_PATH_KEY]: "$",
    [props.GROUP_NAME]: "Employee",
    [FIELDS_KEY]: [
      {
        ID: fldPrimary("$['group:fields'][*].ID[*]", "ID", "Number"),
        Profile: grpSingle("$['group:fields'][*].Profile[*]", "Profile", [["Name", fld("$['group:fields'][*].Profile[*]['group:fields'][*].Name[*]", "Name", "Text")]]),
      },
    ],
  };
}

// ---------------------------------------------------------------------------
// source data
// ---------------------------------------------------------------------------

function userAlice(): unknown {
  return Object.assign(new UserRow(), {
    ID: [101],
    Name: ["Alice"],
    Email: ["alice@example.com"],
  });
}

function employeeBob(): unknown {
  return Object.assign(new EmployeeRow(), {
    ID: [500],
    Skills: ["Go", "Rust"],
    Profile: [
      Object.assign(new ProfileRow(), {
        Name: ["Bob"],
        Age: [30],
        Address: [
          Object.assign(new AddressRow(), {
            Street: ["123 Tech Ln"],
            City: ["Silicon Valley"],
            ZipCode: ["94000"],
          }),
        ],
      }),
    ],
  });
}

function products(): unknown {
  return [Object.assign(new ProductRow(), { ID: [1], Name: ["Laptop"], Price: [999.99] }), Object.assign(new ProductRow(), { ID: [2], Name: ["Mouse"], Price: [25.5] })];
}

function cartesianEmployees(): unknown {
  return [
    Object.assign(new EmployeeRow(), {
      ID: [100],
      Skills: ["Go"],
      Profile: [
        Object.assign(new ProfileRow(), {
          Name: ["Dev"],
          Age: [30],
          Address: [
            Object.assign(new AddressRow(), {
              Street: ["Home St"],
              City: ["Nairobi"],
              ZipCode: ["00100"],
            }),
            Object.assign(new AddressRow(), {
              Street: ["Work Ave"],
              City: ["Westlands"],
              ZipCode: ["00200"],
            }),
          ],
        }),
      ],
    }),
    Object.assign(new EmployeeRow(), {
      ID: [200],
      Skills: ["Management"],
      Profile: [
        Object.assign(new ProfileRow(), {
          Name: ["Admin"],
          Age: [40],
          Address: [
            Object.assign(new AddressRow(), {
              Street: ["Office Rd"],
              City: ["Mombasa"],
              ZipCode: ["80100"],
            }),
          ],
        }),
        Object.assign(new ProfileRow(), {
          Name: ["Consultant"],
          Age: [35],
          Address: [
            Object.assign(new AddressRow(), {
              Street: ["Remote"],
              City: ["Eldoret"],
              ZipCode: ["30100"],
            }),
          ],
        }),
      ],
    }),
  ];
}

function userWithBio(): unknown {
  return Object.assign(new UserWithBio(), { ID: [7], Bio: true });
}

function employeeSingleProfile(): unknown {
  const profile = Object.assign(new ProfileRow(), { Name: ["Solo"] });
  return Object.assign(new EmployeeSingleProfile(), { ID: [9], Profile: profile });
}

// ---------------------------------------------------------------------------
// expected-table helpers
// ---------------------------------------------------------------------------

/** A cell is always an array (the flattener's array formality). */
function cell(items: Value[]): Value {
  return items;
}

function cells(values: number[]): Value {
  return cell(values.map((v) => v));
}

function strCells(values: string[]): Value {
  return cell(values.map((v) => v));
}

function table(rows: Value[][]): Value {
  return rows.map((row) => row.map((c) => c));
}

function tableOf(result: FlattenedTable): Value {
  return table(result);
}

// ---------------------------------------------------------------------------
// cases
// ---------------------------------------------------------------------------

interface FlattenCase {
  testTitle?: string;
  model: () => MetadataModel;
  source: () => unknown;
  expected: Value;
}

function yieldTestData(): FlattenCase[] {
  return [
    // go case 1: simple flat user
    {
      model: userModel,
      source: userAlice,
      expected: table([[cells([101]), strCells(["Alice"]), strCells(["alice@example.com"])]]),
      testTitle: "simple user flattens to one row of array cells (go case 1)",
    },
    // go case 2: deep nested employee
    {
      model: employeeModel,
      source: employeeBob,
      expected: table([[cells([500]), strCells(["Bob"]), cells([30]), strCells(["123 Tech Ln"]), strCells(["Silicon Valley"]), strCells(["94000"]), strCells(["Go", "Rust"])]]),
      testTitle: "deep nested employee flattens to one row, 7 columns (go case 2)",
    },
    // go case 3: array of products
    {
      model: productModel,
      source: products,
      expected: table([
        [cells([1]), strCells(["Laptop"]), cell([999.99])],
        [cells([2]), strCells(["Mouse"]), cell([25.5])],
      ]),
      testTitle: "array of products flattens to one row per product (go case 3)",
    },
    // go case 5: cartesian product across profiles and addresses
    {
      model: employeeModel,
      source: cartesianEmployees,
      expected: table([
        [cells([100]), strCells(["Dev"]), cells([30]), strCells(["Home St"]), strCells(["Nairobi"]), strCells(["00100"]), strCells(["Go"])],
        [cells([100]), strCells(["Dev"]), cells([30]), strCells(["Work Ave"]), strCells(["Westlands"]), strCells(["00200"]), strCells(["Go"])],
        [cells([200]), strCells(["Admin"]), cells([40]), strCells(["Office Rd"]), strCells(["Mombasa"]), strCells(["80100"]), strCells(["Management"])],
        [cells([200]), strCells(["Consultant"]), cells([35]), strCells(["Remote"]), strCells(["Eldoret"]), strCells(["30100"]), strCells(["Management"])],
      ]),
      testTitle: "multiple employees/profiles/addresses produce the cartesian product of 4 rows (go case 5)",
    },
    // single-cardinality scalar field: cell wraps the scalar
    {
      model: userBioModel,
      source: userWithBio,
      expected: table([[cells([7]), cell([true])]]),
      testTitle: "single-cardinality field reads the scalar and wraps it in a cell array",
    },
    // single-cardinality group: no array branching, children read under the object
    {
      model: employeeSingleProfileModel,
      source: employeeSingleProfile,
      expected: table([[cells([9]), strCells(["Solo"])]]),
      testTitle: "single-cardinality group flattens without branching (group [*] dropped)",
    },
  ];
}

test("flatten_cases", () => {
  const report = new TestReport("flatten_cases", "flattener: objects/collections to 2D table (src/flattener)");
  for (const testCase of yieldTestData()) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const model = testCase.model();
    const source = testCase.source();
    try {
      const flattener = new Flattener(model);
      flattener.flatten(DynObject.new(source));
      const got = tableOf(flattener.result());
      if (!areEqual(got, testCase.expected)) {
        report.failCase(`table: expected ${JSON.stringify(testCase.expected)}, got ${JSON.stringify(got)}`);
      }
    } catch (err) {
      report.failCase(`flatten failed: ${err instanceof Error ? err.message : String(err)}`);
    }
  }
  report.finish();
});

test("write_to_destination_case", () => {
  const report = new TestReport("write_to_destination_case", "flattener writeToDestination honors the unskipped column read order (src/flattener)");
  report.beginCase("products table writes into a pre-sized 2D destination");

  const model = productModel();
  const source = products();
  try {
    const flattener = new Flattener(model);
    flattener.flatten(DynObject.new(source));

    // pre-size the destination 2 rows x 3 cols
    const destination: Value = [
      [null, null, null],
      [null, null, null],
    ];
    flattener.writeToDestination(DynObject.new(destination));
    const expected = tableOf(flattener.result());
    if (!areEqual(destination, expected)) {
      report.failCase(`destination: expected ${JSON.stringify(expected)}, got ${JSON.stringify(destination)}`);
    }
  } catch (err) {
    report.failCase(`writeToDestination failed: ${err instanceof Error ? err.message : String(err)}`);
  }
  report.finish();
});
