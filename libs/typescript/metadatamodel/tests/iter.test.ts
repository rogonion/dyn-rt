// Data-driven suite for the metadatamodel `iter` subdomain
// (ADR 0004/0009): src/iter/{domain,application}.ts — recursive forEach /
// filter / map walks over the model's fields/groups (port of
// go-metadatamodel/iter, whose tests use a UserInformation model with a
// nested Details group). Port of libs/rust/metadatamodel/tests/iter/main.rs.

import { test } from "vitest";
import { TestReport, newTestTitle } from "@dyn-rt/test-support";
import { areEqual } from "@dyn-rt/manipulation";
import { object, type Value } from "@dyn-rt/shared";
import { FIELDS_KEY, fieldsOf, isGroup, props, propsOf, setProp, type MetadataModel } from "../src/core/index.js";
import { FilterDecision, Visit, filter, forEach, map } from "../src/iter/index.js";

// ---------------------------------------------------------------------------
// sample model (port of go iter's UserInformationMetadataModel, with
// section:key props and fields-as-arrays)
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

function fldDepth(pathKey: string, name: string, dataType: string, depth: number): Value {
  return [
    {
      [props.GROUP_PATH_KEY]: pathKey,
      [props.GROUP_NAME]: name,
      [props.FIELD_DATA_TYPE]: dataType,
      [props.DB_JOIN_DEPTH]: depth,
    },
  ];
}

function grp(pathKey: string, name: string, fields: [string, Value][], order: string[]): Value {
  return [
    {
      [props.GROUP_PATH_KEY]: pathKey,
      [props.GROUP_NAME]: name,
      [FIELDS_KEY]: [object(fields)],
      [props.GROUP_READ_ORDER_OF_FIELDS]: [...order],
    },
  ];
}

/** Root group "Root Group" with fields ID, Name and nested group Details
 * (City, ZipCode) — port of go iter's UserInformationMetadataModel. */
function userInformationModel(): MetadataModel {
  return {
    [props.GROUP_PATH_KEY]: "$",
    [props.GROUP_NAME]: "Root Group",
    [FIELDS_KEY]: [
      {
        ID: fldPrimary("$['group:fields'][*].ID[*]", "Primary ID", "Text"),
        Name: fld("$['group:fields'][*].Name[*]", "User Name", "Text"),
        Details: grp(
          "$['group:fields'][*].Details[*]",
          "Address Details",
          [
            ["City", fldDepth("$['group:fields'][*].Details[*]['group:fields'][*].City[*]", "City Name", "Text", 1)],
            ["ZipCode", fldDepth("$['group:fields'][*].Details[*]['group:fields'][*].ZipCode[*]", "Postal Code", "Number", 1)],
          ],
          ["City", "ZipCode"],
        ),
      },
    ],
    [props.GROUP_READ_ORDER_OF_FIELDS]: ["ID", "Name", "Details"],
  };
}

/** The model after removing fields whose name ends with "Name" (go filter
 * test case 2): root keeps ID + Details, Details keeps ZipCode. */
function filteredNoNameSuffix(): MetadataModel {
  return {
    [props.GROUP_PATH_KEY]: "$",
    [props.GROUP_NAME]: "Root Group",
    [FIELDS_KEY]: [
      {
        ID: fldPrimary("$['group:fields'][*].ID[*]", "Primary ID", "Text"),
        Details: grp(
          "$['group:fields'][*].Details[*]",
          "Address Details",
          [["ZipCode", fldDepth("$['group:fields'][*].Details[*]['group:fields'][*].ZipCode[*]", "Postal Code", "Number", 1)]],
          ["ZipCode"],
        ),
      },
    ],
    [props.GROUP_READ_ORDER_OF_FIELDS]: ["ID", "Details"],
  };
}

/** The model after removing all groups (go filter test case 3). */
function filteredNoGroups(): MetadataModel {
  return {
    [props.GROUP_PATH_KEY]: "$",
    [props.GROUP_NAME]: "Root Group",
    [FIELDS_KEY]: [
      {
        ID: fldPrimary("$['group:fields'][*].ID[*]", "Primary ID", "Text"),
        Name: fld("$['group:fields'][*].Name[*]", "User Name", "Text"),
      },
    ],
    [props.GROUP_READ_ORDER_OF_FIELDS]: ["ID", "Name"],
  };
}

/** The model after appending " Found" to names ending with "Name" (go map
 * test case 2). */
function mappedNameSuffixFound(): MetadataModel {
  return {
    [props.GROUP_PATH_KEY]: "$",
    [props.GROUP_NAME]: "Root Group",
    [FIELDS_KEY]: [
      {
        ID: fldPrimary("$['group:fields'][*].ID[*]", "Primary ID", "Text"),
        Name: fld("$['group:fields'][*].Name[*]", "User Name Found", "Text"),
        Details: grp(
          "$['group:fields'][*].Details[*]",
          "Address Details",
          [
            ["City", fldDepth("$['group:fields'][*].Details[*]['group:fields'][*].City[*]", "City Name Found", "Text", 1)],
            ["ZipCode", fldDepth("$['group:fields'][*].Details[*]['group:fields'][*].ZipCode[*]", "Postal Code", "Number", 1)],
          ],
          ["City", "ZipCode"],
        ),
      },
    ],
    [props.GROUP_READ_ORDER_OF_FIELDS]: ["ID", "Name", "Details"],
  };
}

/** The model after appending " Found" to group names (go map test case 3). */
function mappedGroupNameFound(): MetadataModel {
  const model = userInformationModel();
  const fieldsObj = fieldsOf(model);
  const details = fieldsObj["Details"];
  if (details !== undefined) {
    setProp(propsOf(details), props.GROUP_NAME, "Address Details Found");
  }
  return model;
}

// ---------------------------------------------------------------------------
// cases
// ---------------------------------------------------------------------------

type IterOp =
  | { kind: "forEachCount"; expected: number }
  | { kind: "forEachTerminateAfter"; n: number; expected: number }
  | { kind: "forEachSkipGroups"; expected: number }
  | { kind: "forEachNames"; expected: string[] }
  | { kind: "filterKeepAll" }
  | { kind: "filterRemoveNameSuffix" }
  | { kind: "filterRemoveGroups" }
  | { kind: "mapIdentity" }
  | { kind: "mapNameSuffixFound" }
  | { kind: "mapGroupNameFound" };

interface IterCase {
  testTitle?: string;
  op: IterOp;
}

function yieldTestData(): IterCase[] {
  return [
    {
      op: { kind: "forEachCount", expected: 5 },
      testTitle: "for_each visits all 5 fields/groups incl. nested (go ForEach case)",
    },
    {
      op: { kind: "forEachTerminateAfter", n: 2, expected: 2 },
      testTitle: "for_each terminates when the callback says so",
    },
    {
      op: { kind: "forEachSkipGroups", expected: 3 },
      testTitle: "for_each with skipFields on groups does not descend",
    },
    {
      op: {
        kind: "forEachNames",
        expected: ["Primary ID", "User Name", "Address Details", "City Name", "Postal Code"],
      },
      testTitle: "for_each visits in read order, depth-first",
    },
    {
      op: { kind: "filterKeepAll" },
      testTitle: "filter keep everything leaves the model unchanged (go Filter case 1)",
    },
    {
      op: { kind: "filterRemoveNameSuffix" },
      testTitle: "filter removes fields whose name ends with 'Name' (go Filter case 2)",
    },
    {
      op: { kind: "filterRemoveGroups" },
      testTitle: "filter removes groups, keeping plain fields (go Filter case 3)",
    },
    {
      op: { kind: "mapIdentity" },
      testTitle: "map identity leaves the model unchanged (go Map case 1)",
    },
    {
      op: { kind: "mapNameSuffixFound" },
      testTitle: "map appends ' Found' to names ending with 'Name' (go Map case 2)",
    },
    {
      op: { kind: "mapGroupNameFound" },
      testTitle: "map appends ' Found' to group names (go Map case 3)",
    },
  ];
}

// ---------------------------------------------------------------------------
// callback helpers
// ---------------------------------------------------------------------------

/** The props of `props` with `group:name` appended with `suffix` — helper
 * for the map callbacks; returns the new ENTRY (array with the props). */
function renamedEntry(propsValue: Value, suffix: string): Value {
  const name = typeof propsValue === "object" && propsValue !== null && !Array.isArray(propsValue) ? propsValue[props.GROUP_NAME] : undefined;
  const newProps: Value = typeof name === "string" ? { ...(propsValue as Record<string, Value>), [props.GROUP_NAME]: `${name}${suffix}` } : propsValue;
  return [newProps];
}

/** Rebuilds the entry (array with the props) unchanged — the map callback's
 * "keep as-is" return. */
function propsEntry(propsValue: Value): Value {
  return [propsValue];
}

function nameOf(propsValue: Value): string | undefined {
  return typeof propsValue === "object" && propsValue !== null && !Array.isArray(propsValue) ? (propsValue[props.GROUP_NAME] as string | undefined) : undefined;
}

function expectedModel(op: IterOp): MetadataModel {
  switch (op.kind) {
    case "filterKeepAll":
    case "mapIdentity":
      return userInformationModel();
    case "filterRemoveNameSuffix":
      return filteredNoNameSuffix();
    case "filterRemoveGroups":
      return filteredNoGroups();
    case "mapNameSuffixFound":
      return mappedNameSuffixFound();
    case "mapGroupNameFound":
      return mappedGroupNameFound();
    default:
      throw new Error(`no expected model for op ${op.kind}`);
  }
}

test("iter_cases", () => {
  const report = new TestReport("iter_cases", "iter forEach/filter/map walks (src/iter/application.ts)");
  for (const testCase of yieldTestData()) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const op = testCase.op;
    try {
      if (op.kind === "forEachCount") {
        let count = 0;
        forEach(userInformationModel(), () => {
          count += 1;
          return Visit.CONTINUE;
        });
        if (count !== op.expected) {
          report.failCase(`count: expected ${op.expected}, got ${count}`);
        }
      } else if (op.kind === "forEachTerminateAfter") {
        let count = 0;
        forEach(userInformationModel(), () => {
          count += 1;
          return count >= op.n ? Visit.TERMINATE : Visit.CONTINUE;
        });
        if (count !== op.expected) {
          report.failCase(`count: expected ${op.expected}, got ${count}`);
        }
      } else if (op.kind === "forEachSkipGroups") {
        let count = 0;
        forEach(userInformationModel(), (propsValue) => {
          count += 1;
          return { terminate: false, skipFields: isGroup(propsValue) };
        });
        if (count !== op.expected) {
          report.failCase(`count: expected ${op.expected}, got ${count}`);
        }
      } else if (op.kind === "forEachNames") {
        const names: string[] = [];
        forEach(userInformationModel(), (propsValue) => {
          names.push(nameOf(propsValue) ?? "<missing>");
          return Visit.CONTINUE;
        });
        if (JSON.stringify(names) !== JSON.stringify(op.expected)) {
          report.failCase(`names: expected ${JSON.stringify(op.expected)}, got ${JSON.stringify(names)}`);
        }
      } else if (op.kind.startsWith("filter")) {
        const model = userInformationModel();
        filter(model, (propsValue) => {
          let retain = true;
          if (op.kind === "filterRemoveNameSuffix") {
            retain = !(nameOf(propsValue)?.endsWith("Name") ?? false);
          } else if (op.kind === "filterRemoveGroups") {
            retain = !isGroup(propsValue);
          }
          return { retain, skipFields: false } satisfies FilterDecision;
        });
        const expected = expectedModel(op);
        if (!areEqual(model, expected)) {
          report.failCase(`model: expected ${JSON.stringify(expected)}, got ${JSON.stringify(model)}`);
        }
      } else {
        // map ops
        const model = userInformationModel();
        map(model, (propsValue) => {
          if (op.kind === "mapNameSuffixFound") {
            const name = nameOf(propsValue);
            return [name?.endsWith("Name") ? renamedEntry(propsValue, " Found") : propsEntry(propsValue), false];
          }
          if (op.kind === "mapGroupNameFound") {
            return [isGroup(propsValue) ? renamedEntry(propsValue, " Found") : propsEntry(propsValue), false];
          }
          return [propsEntry(propsValue), false];
        });
        const expected = expectedModel(op);
        if (!areEqual(model, expected)) {
          report.failCase(`model: expected ${JSON.stringify(expected)}, got ${JSON.stringify(model)}`);
        }
      }
    } catch (err) {
      report.failCase(`${op.kind} failed: ${err instanceof Error ? err.message : String(err)}`);
    }
  }
  report.finish();
});
