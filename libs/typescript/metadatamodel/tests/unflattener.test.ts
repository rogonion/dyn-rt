// Data-driven suites for the metadatamodel `unflattener` subdomain
// (ADR 0004/0009): src/unflattener/{domain,application}.ts — converting a
// 2D table back into a nested collection (port of
// go-metadatamodel/unflattener's unflatten_test.go), incl.
// cardinality-aware writes (single-cardinality unwrap, multi-value error /
// take-first) and a flatten->unflatten round-trip.
// Port of libs/rust/metadatamodel/tests/unflattener/main.rs.

import { test } from "vitest";
import { TestReport, newTestTitle } from "@dyn-rt/test-support";
import { areEqual, DynObject } from "@dyn-rt/manipulation";
import { classNode, scalar, sequenceNode, type SchemaNode } from "@dyn-rt/schema";
import { object, type Value } from "@dyn-rt/shared";
import { FIELDS_KEY, props, type MetadataModel } from "../src/core/index.js";
import { Flattener, toValue } from "../src/flattener/index.js";
import { Unflattener, type SingleCellMode } from "../src/unflattener/index.js";

// ---------------------------------------------------------------------------
// destination shapes (same as the flattener suite's source shapes)
// ---------------------------------------------------------------------------

class UserRow {
  ID: number[] = [];
  Name: string[] = [];
  Email: string[] = [];
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

class UserWithBio {
  ID: number[] = [];
  Bio = false;
}

// ---------------------------------------------------------------------------
// schemas for the destinations
// ---------------------------------------------------------------------------

function intSeq(): SchemaNode {
  return sequenceNode(scalar("i64"));
}

function strSeq(): SchemaNode {
  return sequenceNode(scalar("string"));
}

function userSchema(): SchemaNode {
  return sequenceNode(
    classNode("UserRow", [
      ["ID", intSeq()],
      ["Name", strSeq()],
      ["Email", strSeq()],
    ]),
  );
}

function addressSchema(): SchemaNode {
  return classNode("AddressRow", [
    ["Street", strSeq()],
    ["City", strSeq()],
    ["ZipCode", strSeq()],
  ]);
}

function profileSchema(): SchemaNode {
  return classNode("ProfileRow", [
    ["Name", strSeq()],
    ["Age", intSeq()],
    ["Address", sequenceNode(addressSchema())],
  ]);
}

function employeeSchema(): SchemaNode {
  return sequenceNode(
    classNode("EmployeeRow", [
      ["ID", intSeq()],
      ["Skills", strSeq()],
      ["Profile", sequenceNode(profileSchema())],
    ]),
  );
}

function userBioSchema(): SchemaNode {
  return sequenceNode(
    classNode("UserWithBio", [
      ["ID", intSeq()],
      ["Bio", scalar("bool")],
    ]),
  );
}

// ---------------------------------------------------------------------------
// sample models (same shapes as the flattener suite)
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

// ---------------------------------------------------------------------------
// table helpers (cells are arrays — the flattener's output formality)
// ---------------------------------------------------------------------------

function cellsI(values: number[]): Value {
  return values.map((v) => v);
}

function cellsS(values: string[]): Value {
  return values.map((v) => v);
}

// ---------------------------------------------------------------------------
// cases
//
// NB: destination comparison normalizes both sides through the flattener's
// toValue — schema-driven `set` builds plain objects when the destination
// starts empty, while the expected fixtures are class instances, and
// areEqual distinguishes the two kinds.
// ---------------------------------------------------------------------------

type UnflattenOp =
  | { kind: "users"; table: Value[][]; expected: unknown }
  | { kind: "employees"; table: Value[][]; expected: unknown }
  | {
      kind: "bio";
      table: Value[][];
      expected?: unknown;
      expectError?: string;
      mode: SingleCellMode;
    };

interface UnflattenCase {
  testTitle?: string;
  model: () => MetadataModel;
  op: UnflattenOp;
}

function yieldTestData(): UnflattenCase[] {
  return [
    // go case 1: simple user
    {
      model: userModel,
      op: {
        kind: "users",
        table: [[cellsI([101]), cellsS(["Alice"]), cellsS(["alice@example.com"])]],
        expected: [
          Object.assign(new UserRow(), {
            ID: [101],
            Name: ["Alice"],
            Email: ["alice@example.com"],
          }),
        ],
      },
      testTitle: "simple user table unflattens to a one-element collection (go case 1)",
    },
    // go case 2-like: nested employee, one row
    {
      model: employeeModel,
      op: {
        kind: "employees",
        table: [[cellsI([500]), cellsS(["Bob"]), cellsI([30]), cellsS(["123 Tech Ln"]), cellsS(["Silicon Valley"]), cellsS(["94000"]), cellsS(["Go", "Rust"])]],
        expected: [
          Object.assign(new EmployeeRow(), {
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
          }),
        ],
      },
      testTitle: "nested employee row unflattens into nested groups (go case 2)",
    },
    // cartesian table: rows merge into instances by signature
    {
      model: employeeModel,
      op: {
        kind: "employees",
        table: [
          [cellsI([100]), cellsS(["Dev"]), cellsI([30]), cellsS(["Home St"]), cellsS(["Nairobi"]), cellsS(["00100"]), cellsS(["Go"])],
          [cellsI([100]), cellsS(["Dev"]), cellsI([30]), cellsS(["Work Ave"]), cellsS(["Westlands"]), cellsS(["00200"]), cellsS(["Go"])],
          [cellsI([200]), cellsS(["Admin"]), cellsI([40]), cellsS(["Office Rd"]), cellsS(["Mombasa"]), cellsS(["80100"]), cellsS(["Management"])],
          [cellsI([200]), cellsS(["Consultant"]), cellsI([35]), cellsS(["Remote"]), cellsS(["Eldoret"]), cellsS(["30100"]), cellsS(["Management"])],
        ],
        expected: [
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
        ],
      },
      testTitle: "cartesian table merges rows into instances by signature (go case 5 inverse)",
    },
    // single-cardinality field: cell array unwrapped to the scalar
    {
      model: userBioModel,
      op: {
        kind: "bio",
        table: [[cellsI([7]), [true]]],
        expected: [Object.assign(new UserWithBio(), { ID: [7], Bio: true })],
        mode: "Error",
      },
      testTitle: "single-cardinality field: cell array unwrapped to the scalar",
    },
    // single-cardinality field with >1 values: error in strict mode
    {
      model: userBioModel,
      op: {
        kind: "bio",
        table: [[cellsI([7]), [true, false]]],
        expectError: "single-cardinality",
        mode: "Error",
      },
      testTitle: "single-cardinality field with 2 values errors in strict mode",
    },
    // same but TakeFirst
    {
      model: userBioModel,
      op: {
        kind: "bio",
        table: [[cellsI([7]), [true, false]]],
        expected: [Object.assign(new UserWithBio(), { ID: [7], Bio: true })],
        mode: "TakeFirst",
      },
      testTitle: "single-cardinality field with 2 values takes the first in TakeFirst mode",
    },
  ];
}

test("unflatten_cases", () => {
  const report = new TestReport("unflatten_cases", "unflattener: 2D table to nested collection (src/unflattener)");
  for (const testCase of yieldTestData()) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const model = testCase.model();
    const op = testCase.op;
    try {
      if (op.kind === "users") {
        const dest: UserRow[] = [];
        const unflattener = new Unflattener(model, DynObject.new(dest).withSchemaOpt(userSchema()));
        unflattener.unflatten(op.table);
        if (!areEqual(toValue(dest), toValue(op.expected))) {
          report.failCase(`dest: expected ${JSON.stringify(op.expected)}, got ${JSON.stringify(dest)}`);
        }
      } else if (op.kind === "employees") {
        const dest: EmployeeRow[] = [];
        const unflattener = new Unflattener(model, DynObject.new(dest).withSchemaOpt(employeeSchema()));
        unflattener.unflatten(op.table);
        if (!areEqual(toValue(dest), toValue(op.expected))) {
          report.failCase(`dest: expected ${JSON.stringify(op.expected)}, got ${JSON.stringify(dest)}`);
        }
      } else {
        const dest: UserWithBio[] = [];
        const unflattener = new Unflattener(model, DynObject.new(dest).withSchemaOpt(userBioSchema()));
        unflattener.singleCellMode = op.mode;
        try {
          unflattener.unflatten(op.table);
          if (op.expectError !== undefined) {
            report.failCase(`expected error containing ${JSON.stringify(op.expectError)}, got Ok`);
          } else if (op.expected !== undefined && !areEqual(toValue(dest), toValue(op.expected))) {
            report.failCase(`dest: expected ${JSON.stringify(op.expected)}, got ${JSON.stringify(dest)}`);
          } else if (op.expected === undefined && op.expectError === undefined) {
            report.failCase("expected an error but got Ok");
          }
        } catch (err) {
          if (op.expectError !== undefined) {
            const message = err instanceof Error ? err.message : String(err);
            if (!message.includes(op.expectError)) {
              report.failCase(`error expected to contain ${JSON.stringify(op.expectError)}, got ${message}`);
            }
          } else {
            report.failCase(`unexpected error: ${err instanceof Error ? err.message : String(err)}`);
          }
        }
      }
    } catch (err) {
      report.failCase(`unexpected error: ${err instanceof Error ? err.message : String(err)}`);
    }
  }
  report.finish();
});

/** End-to-end: flatten an EmployeeRow[], then unflatten the table back and
 * expect the original collection. */
test("flatten_unflatten_roundtrip_case", () => {
  const report = new TestReport("flatten_unflatten_roundtrip_case", "flatten then unflatten reproduces the source collection (src/flattener + src/unflattener)");
  report.beginCase("cartesian employees round-trip");

  const model = employeeModel();
  const source: EmployeeRow[] = [
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
      ],
    }),
  ];

  try {
    const flattener = new Flattener(model);
    flattener.flatten(DynObject.new(source));
    const table = flattener.result();

    const dest: EmployeeRow[] = [];
    const unflattener = new Unflattener(model, DynObject.new(dest).withSchemaOpt(employeeSchema()));
    unflattener.unflatten(table);
    if (!areEqual(toValue(dest), toValue(source))) {
      report.failCase(`round-trip: expected ${JSON.stringify(source)}, got ${JSON.stringify(dest)}`);
    }
  } catch (err) {
    report.failCase(`round-trip failed: ${err instanceof Error ? err.message : String(err)}`);
  }
  report.finish();
});
