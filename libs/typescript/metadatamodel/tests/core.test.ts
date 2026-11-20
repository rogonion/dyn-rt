// Data-driven suites for the metadatamodel `core` subdomain (ADR 0004/0009):
// src/core/domain.ts + src/core/application.ts — property getters, the
// templated `group:path_key` post-processing (schema path vs data path), and
// the strict cardinality rule. Port of libs/rust/metadatamodel/tests/core/main.rs.

import { test } from "vitest";
import { TestReport, newTestTitle } from "@dyn-rt/test-support";
import type { Value } from "@dyn-rt/shared";
import {
  FIELDS_KEY,
  props,
  cardinalityOf,
  checkValueCardinality,
  containsNestedGroups,
  dataPath,
  dataPathKeepWildcards,
  dataPathModelAware,
  fieldsOf,
  nameOf,
  parseCardinality,
  pathKeyOf,
  prop,
  propsOf,
  readOrderOf,
  schemaPath,
  type Cardinality,
  type MetadataModel,
} from "../src/core/index.js";

// ---------------------------------------------------------------------------
// template path cases
// ---------------------------------------------------------------------------

type TemplateOp =
  | { kind: "schemaPath"; indices: number[] }
  | { kind: "dataPath"; indices: number[]; cardinality: Cardinality }
  | {
      kind: "dataPathModelAware";
      model: MetadataModel;
      indices: number[];
      indexLastWildcard: boolean;
    }
  | { kind: "dataPathKeepWildcards" };

interface TemplateCase {
  testTitle?: string;
  template: string;
  op: TemplateOp;
  expected?: string;
  expectError?: string;
}

function yieldTemplateData(): TemplateCase[] {
  return [
    {
      // schema path: every [*] replaced by an index, nothing trimmed
      testTitle: "schema path of a field substitutes all wildcards",
      template: "$['group:fields'][*].Bio[*]",
      op: { kind: "schemaPath", indices: [0, 0] },
      expected: "$['group:fields'][0].Bio[0]",
    },
    {
      testTitle: "schema path of a nested group field",
      template: "$['group:fields'][*].Site[*]['group:fields'][*].Country[*]",
      op: { kind: "schemaPath", indices: [0, 1, 0, 2] },
      expected: "$['group:fields'][0].Site[1]['group:fields'][0].Country[2]",
    },
    {
      // data path, array cardinality: group:fields + its [*] trimmed,
      // remaining [*] indexed
      testTitle: "data path of an array field trims group:fields and indexes the value",
      template: "$['group:fields'][*].Bio[*]",
      op: { kind: "dataPath", indices: [2], cardinality: "array" },
      expected: "$.Bio[2]",
    },
    {
      // data path, single cardinality: trailing [*] trimmed, no index
      testTitle: "data path of a single field trims the trailing [*]",
      template: "$['group:fields'][*].Bio[*]",
      op: { kind: "dataPath", indices: [], cardinality: "single" },
      expected: "$.Bio",
    },
    {
      testTitle: "data path of a nested array field keeps group entry and value indices",
      template: "$['group:fields'][*].Site[*]['group:fields'][*].Country[*]",
      op: { kind: "dataPath", indices: [1, 2], cardinality: "array" },
      expected: "$.Site[1].Country[2]",
    },
    {
      testTitle: "root template data path is the root",
      template: "$",
      op: { kind: "dataPath", indices: [], cardinality: "array" },
      expected: "$",
    },
    {
      // go behavior: missing indices default to 0
      testTitle: "schema path defaults missing indices to 0",
      template: "$['group:fields'][*].Bio[*]",
      op: { kind: "schemaPath", indices: [0] },
      expected: "$['group:fields'][0].Bio[0]",
    },
    {
      testTitle: "data path defaults a missing trailing index to 0",
      template: "$['group:fields'][*].Site[*]['group:fields'][*].Country[*]",
      op: { kind: "dataPath", indices: [1], cardinality: "array" },
      expected: "$.Site[1].Country[0]",
    },
    {
      testTitle: "keep-wildcards strips group:fields but leaves [*] in place",
      template: "$['group:fields'][*].Site[*]['group:fields'][*].Country[*]",
      op: { kind: "dataPathKeepWildcards" },
      expected: "$.Site.*.Country.*",
    },
    {
      testTitle: "model-aware data path indexes array fields and drops single ones",
      template: "$['group:fields'][*].Site[*]['group:fields'][*].Country[*]",
      op: {
        kind: "dataPathModelAware",
        model: sampleModel(),
        indices: [1, 2],
        indexLastWildcard: true,
      },
      expected: "$.Site[1].Country[2]",
    },
    {
      testTitle: "model-aware data path with indexLastWildcard=false reads the whole leaf value",
      template: "$['group:fields'][*].Bio[*]",
      op: {
        kind: "dataPathModelAware",
        model: sampleModel(),
        indices: [],
        indexLastWildcard: false,
      },
      expected: "$.Bio",
    },
    {
      testTitle: "model-aware data path drops the single SourceId trailing wildcard",
      template: "$['group:fields'][*].SourceId[*]",
      op: {
        kind: "dataPathModelAware",
        model: sampleModel(),
        indices: [],
        indexLastWildcard: true,
      },
      expected: "$.SourceId",
    },
  ];
}

test("template_cases", () => {
  const report = new TestReport("template_cases", "group:path_key template post-processing: schemaPath / dataPath (src/core/application.ts)");
  for (const testCase of yieldTemplateData()) {
    report.beginCase(newTestTitle(testCase.testTitle));
    let got: string | undefined;
    let errorMessage = "";
    try {
      switch (testCase.op.kind) {
        case "schemaPath":
          got = schemaPath(testCase.template, testCase.op.indices);
          break;
        case "dataPath":
          got = dataPath(testCase.template, testCase.op.indices, testCase.op.cardinality);
          break;
        case "dataPathModelAware":
          got = dataPathModelAware(testCase.template, testCase.op.model, testCase.op.indices, testCase.op.indexLastWildcard);
          break;
        case "dataPathKeepWildcards":
          got = dataPathKeepWildcards(testCase.template);
          break;
      }
    } catch (err) {
      errorMessage = err instanceof Error ? err.message : String(err);
    }
    if (testCase.expectError !== undefined) {
      if (got !== undefined || !errorMessage.includes(testCase.expectError)) {
        report.failCase(`expected error containing ${JSON.stringify(testCase.expectError)}, got ${got !== undefined ? JSON.stringify(got) : JSON.stringify(errorMessage)}`);
      }
      continue;
    }
    if (got === undefined) {
      report.failCase(`unexpected error: ${errorMessage}`);
    } else if (got !== testCase.expected) {
      report.failCase(`template ${testCase.template} || expected ${JSON.stringify(testCase.expected)}, got ${JSON.stringify(got)}`);
    }
  }
  report.finish();
});

// ---------------------------------------------------------------------------
// model property cases
// ---------------------------------------------------------------------------

/** A small metadata model: root group with fields SourceId (single) and
 * Bio (array, default), and a nested group Site with field Country. */
function sampleModel(): MetadataModel {
  return {
    [props.GROUP_PATH_KEY]: "$",
    [props.GROUP_NAME]: "Vector Data",
    [FIELDS_KEY]: [
      {
        SourceId: [
          {
            [props.GROUP_PATH_KEY]: "$['group:fields'][*].SourceId[*]",
            [props.GROUP_NAME]: "source id",
            [props.FIELD_DATA_TYPE]: "Number",
            [props.FIELD_CARDINALITY]: "single",
          },
        ],
        Bio: [
          {
            [props.GROUP_PATH_KEY]: "$['group:fields'][*].Bio[*]",
            [props.FIELD_DATA_TYPE]: "Boolean",
          },
        ],
        Site: [
          {
            [props.GROUP_PATH_KEY]: "$['group:fields'][*].Site[*]",
            [props.GROUP_CARDINALITY]: "array",
            [FIELDS_KEY]: [
              {
                Country: [
                  {
                    [props.GROUP_PATH_KEY]: "$['group:fields'][*].Site[*]['group:fields'][*].Country[*]",
                    [props.FIELD_DATA_TYPE]: "Text",
                  },
                ],
              },
            ],
          },
        ],
      },
    ],
  };
}

type ModelOp =
  | { kind: "pathKeyOf"; expected: string }
  | { kind: "nameOf"; defaultValue: string; expected: string }
  | { kind: "nameOfProps"; fieldGroup: MetadataModel; defaultValue: string; expected: string }
  | { kind: "fieldsContain"; field: string }
  | { kind: "propsDataType"; field: string; expected: string }
  | { kind: "cardinalityOf"; field: string; expected: Cardinality }
  | { kind: "readOrder"; expected: string[] }
  | { kind: "containsNestedGroups"; expected: boolean }
  | { kind: "checkCardinality"; value: Value; cardinality: Cardinality; expectError?: string }
  | { kind: "parseCardinality"; raw: string; expected?: Cardinality };

interface ModelCase {
  testTitle?: string;
  op: ModelOp;
}

function yieldModelData(): ModelCase[] {
  return [
    { testTitle: "root group path key", op: { kind: "pathKeyOf", expected: "$" } },
    {
      testTitle: "name from group:name",
      op: { kind: "nameOf", defaultValue: "", expected: "Vector Data" },
    },
    {
      // templates end in [*], so the fallback needs a key-terminated template
      testTitle: "name falls back to the last template segment",
      op: {
        kind: "nameOfProps",
        fieldGroup: { [props.GROUP_PATH_KEY]: "$['group:fields'][*].Site" },
        defaultValue: "",
        expected: "Site",
      },
    },
    {
      testTitle: "name falls back to the default when nothing else is available",
      op: { kind: "nameOfProps", fieldGroup: {}, defaultValue: "fallback", expected: "fallback" },
    },
    {
      testTitle: "fieldsOf exposes the nested group",
      op: { kind: "fieldsContain", field: "Site" },
    },
    {
      testTitle: "propsOf unwraps the field props array",
      op: { kind: "propsDataType", field: "SourceId", expected: "Number" },
    },
    {
      testTitle: "explicit single cardinality",
      op: { kind: "cardinalityOf", field: "SourceId", expected: "single" },
    },
    {
      testTitle: "cardinality defaults to array",
      op: { kind: "cardinalityOf", field: "Bio", expected: "array" },
    },
    {
      testTitle: "read order defaults to the fields' key order",
      op: { kind: "readOrder", expected: ["SourceId", "Bio", "Site"] },
    },
    {
      testTitle: "containsNestedGroups finds the Site group",
      op: { kind: "containsNestedGroups", expected: true },
    },
    {
      testTitle: "array value satisfies array cardinality",
      op: { kind: "checkCardinality", value: [1], cardinality: "array" },
    },
    {
      testTitle: "single value violates array cardinality (strict)",
      op: {
        kind: "checkCardinality",
        value: 1,
        cardinality: "array",
        expectError: "cardinality violation",
      },
    },
    {
      testTitle: "array value violates single cardinality (strict)",
      op: {
        kind: "checkCardinality",
        value: [1],
        cardinality: "single",
        expectError: "cardinality violation",
      },
    },
    {
      testTitle: "single value satisfies single cardinality",
      op: { kind: "checkCardinality", value: "text", cardinality: "single" },
    },
    {
      testTitle: "parseCardinality maps single/object/array/list",
      op: { kind: "parseCardinality", raw: "object", expected: "single" },
    },
    {
      testTitle: "parseCardinality maps list to array",
      op: { kind: "parseCardinality", raw: "list", expected: "array" },
    },
    {
      testTitle: "parseCardinality rejects unknown values",
      op: { kind: "parseCardinality", raw: "weird" },
    },
  ];
}

test("model_cases", () => {
  const report = new TestReport("model_cases", "model property getters + strict cardinality (src/core/application.ts)");
  for (const testCase of yieldModelData()) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const model = sampleModel();
    const op = testCase.op;
    try {
      switch (op.kind) {
        case "pathKeyOf": {
          const got = pathKeyOf(model);
          if (got !== op.expected) report.failCase(`expected ${JSON.stringify(op.expected)}, got ${JSON.stringify(got)}`);
          break;
        }
        case "nameOf": {
          const got = nameOf(model, op.defaultValue);
          if (got !== op.expected) report.failCase(`expected ${JSON.stringify(op.expected)}, got ${JSON.stringify(got)}`);
          break;
        }
        case "nameOfProps": {
          const got = nameOf(op.fieldGroup, op.defaultValue);
          if (got !== op.expected) report.failCase(`expected ${JSON.stringify(op.expected)}, got ${JSON.stringify(got)}`);
          break;
        }
        case "fieldsContain": {
          if (prop(fieldsOf(model), op.field) === undefined) {
            report.failCase(`fields object has no ${JSON.stringify(op.field)} entry`);
          }
          break;
        }
        case "propsDataType": {
          const entry = prop(fieldsOf(model), op.field);
          if (entry === undefined) {
            report.failCase(`no field entry ${op.field}`);
            break;
          }
          const dataType = prop(propsOf(entry), props.FIELD_DATA_TYPE);
          const got = typeof dataType === "string" ? dataType : "<missing>";
          if (got !== op.expected) report.failCase(`expected ${JSON.stringify(op.expected)}, got ${JSON.stringify(got)}`);
          break;
        }
        case "cardinalityOf": {
          const entry = prop(fieldsOf(model), op.field);
          if (entry === undefined) {
            report.failCase(`no field entry ${op.field}`);
            break;
          }
          const got = cardinalityOf(propsOf(entry));
          if (got !== op.expected) report.failCase(`expected ${op.expected}, got ${got}`);
          break;
        }
        case "readOrder": {
          const got = readOrderOf(model);
          if (got.join(",") !== op.expected.join(",")) {
            report.failCase(`expected ${JSON.stringify(op.expected)}, got ${JSON.stringify(got)}`);
          }
          break;
        }
        case "containsNestedGroups": {
          const got = containsNestedGroups(model);
          if (got !== op.expected) report.failCase(`expected ${op.expected}, got ${got}`);
          break;
        }
        case "checkCardinality": {
          try {
            checkValueCardinality(op.value, op.cardinality);
            if (op.expectError !== undefined) report.failCase(`expected error containing ${op.expectError}, got ok`);
          } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            if (op.expectError === undefined) {
              report.failCase(`unexpected error: ${message}`);
            } else if (!message.includes(op.expectError)) {
              report.failCase(`expected error containing ${JSON.stringify(op.expectError)}, got ${JSON.stringify(message)}`);
            }
          }
          break;
        }
        case "parseCardinality": {
          const got = parseCardinality(op.raw);
          if (got !== op.expected) report.failCase(`expected ${op.expected}, got ${got}`);
          break;
        }
      }
    } catch (err) {
      report.failCase(`unexpected error: ${err instanceof Error ? err.message : String(err)}`);
    }
  }
  report.finish();
});
