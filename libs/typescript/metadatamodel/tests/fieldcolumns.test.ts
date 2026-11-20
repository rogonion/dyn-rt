// Data-driven suite for the metadatamodel `fieldcolumns` subdomain
// (ADR 0004/0009): src/fieldcolumns/{domain,application}.ts — extraction of
// model fields as flat 2D columns, skip, and reposition. Port of
// libs/rust/metadatamodel/tests/fieldcolumns/main.rs.

import { test } from "vitest";
import { TestReport, newTestTitle } from "@dyn-rt/test-support";
import type { Value } from "@dyn-rt/shared";
import { FIELDS_KEY, props, type MetadataModel } from "../src/core/index.js";
import { ColumnFields, PropMatch, columnKeyOf, extract, type ExtractOptions } from "../src/fieldcolumns/index.js";

// ---------------------------------------------------------------------------
// sample model
// ---------------------------------------------------------------------------

function field(pathKey: string, name: string, dataType: string): Value {
  return [
    {
      [props.GROUP_PATH_KEY]: pathKey,
      [props.GROUP_NAME]: name,
      [props.FIELD_DATA_TYPE]: dataType,
    },
  ];
}

/** Root group with read order [SourceId, Bio, Skills, Site, Geo]:
 * - SourceId: plain field
 * - Bio: plain field with view:column_position before SourceId
 * - Skills: field with 2 separate columns (header format "skill [*]")
 * - Site: nested group with fields Country, City
 * - Geo: non-nested group expanded into 2 separate columns (Lat, Lng) */
function sampleModel(): MetadataModel {
  const sourceId = field("$['group:fields'][*].SourceId[*]", "source id", "Number");
  const bio: Value = [
    {
      [props.GROUP_PATH_KEY]: "$['group:fields'][*].Bio[*]",
      [props.GROUP_NAME]: "bio",
      [props.FIELD_DATA_TYPE]: "Boolean",
      [props.VIEW_COLUMN_POSITION]: {
        [props.GROUP_PATH_KEY]: "$['group:fields'][*].SourceId[*]",
        [props.VIEW_POSITION_BEFORE]: true,
      },
    },
  ];
  const skills: Value = [
    {
      [props.GROUP_PATH_KEY]: "$['group:fields'][*].Skills[*]",
      [props.GROUP_NAME]: "skill",
      [props.FIELD_DATA_TYPE]: "Text",
      [props.VIEW_VALUES_IN_SEPARATE_COLUMNS]: true,
      [props.VIEW_MAX_VALUES_IN_SEPARATE_COLUMNS]: 2,
      [props.VIEW_SEPARATE_COLUMNS_HEADER_FORMAT]: "skill [*]",
    },
  ];
  const site: Value = [
    {
      [props.GROUP_PATH_KEY]: "$['group:fields'][*].Site[*]",
      [props.GROUP_NAME]: "site",
      [FIELDS_KEY]: [
        {
          Country: field("$['group:fields'][*].Site[*]['group:fields'][*].Country[*]", "country", "Text"),
          City: field("$['group:fields'][*].Site[*]['group:fields'][*].City[*]", "city", "Text"),
        },
      ],
    },
  ];
  const geo: Value = [
    {
      [props.GROUP_PATH_KEY]: "$['group:fields'][*].Geo[*]",
      [props.GROUP_NAME]: "geo",
      [props.VIEW_VALUES_IN_SEPARATE_COLUMNS]: true,
      [props.VIEW_MAX_VALUES_IN_SEPARATE_COLUMNS]: 2,
      [FIELDS_KEY]: [
        {
          Lat: field("$['group:fields'][*].Geo[*]['group:fields'][*].Lat[*]", "lat", "Number"),
          Lng: field("$['group:fields'][*].Geo[*]['group:fields'][*].Lng[*]", "lng", "Number"),
        },
      ],
    },
  ];

  return {
    [props.GROUP_PATH_KEY]: "$",
    [props.GROUP_NAME]: "root",
    [props.GROUP_READ_ORDER_OF_FIELDS]: ["SourceId", "Bio", "Skills", "Site", "Geo"],
    [FIELDS_KEY]: [{ SourceId: sourceId, Bio: bio, Skills: skills, Site: site, Geo: geo }],
  };
}

// ---------------------------------------------------------------------------
// cases
// ---------------------------------------------------------------------------

type FieldColumnsOp =
  { kind: "extract"; expectedKeys: string[]; expectedNames: string[] } | { kind: "extractSkip"; skip: PropMatch; expectedKeys: string[] } | { kind: "extractReposition"; expectedKeys: string[] };

interface FieldColumnsCase {
  testTitle?: string;
  op: FieldColumnsOp;
}

const ALL_KEYS = [
  "$['group:fields'][*].SourceId[*]",
  "$['group:fields'][*].Bio[*]",
  "$['group:fields'][*].Skills[*][0]",
  "$['group:fields'][*].Skills[*][1]",
  "$['group:fields'][*].Site[*]['group:fields'][*].Country[*]",
  "$['group:fields'][*].Site[*]['group:fields'][*].City[*]",
  "$['group:fields'][*].Geo[*][0].Lat",
  "$['group:fields'][*].Geo[*][0].Lng",
  "$['group:fields'][*].Geo[*][1].Lat",
  "$['group:fields'][*].Geo[*][1].Lng",
];

const ALL_NAMES = ["source id", "bio", "skill 1", "skill 2", "country", "city", "lat 1", "lng 1", "lat 2", "lng 2"];

function yieldTestData(): FieldColumnsCase[] {
  return [
    {
      testTitle: "extract: plain fields, separate-column field, nested group, group-as-columns",
      op: { kind: "extract", expectedKeys: ALL_KEYS, expectedNames: ALL_NAMES },
    },
    {
      testTitle: "skip removes matching columns from the current read order",
      op: {
        kind: "extractSkip",
        skip: new PropMatch([[props.GROUP_NAME, "bio"]]),
        expectedKeys: ALL_KEYS.filter((k) => k !== "$['group:fields'][*].Bio[*]"),
      },
    },
    {
      testTitle: "view:column_position with position_before moves bio before source id",
      op: {
        kind: "extractReposition",
        expectedKeys: ["$['group:fields'][*].Bio[*]", ...ALL_KEYS.filter((k) => k !== "$['group:fields'][*].Bio[*]")],
      },
    },
  ];
}

function columnKeys(fields: ColumnFields, order: number[]): string[] {
  return order.map((original) => columnKeyOf(fields.readOrder[original]!));
}

function columnNames(fields: ColumnFields, order: number[]): string[] {
  return order.map((original) => {
    const field = fields.getByIndexInOriginalReadOrder(original);
    const name = field?.props;
    return ((typeof name === "object" && name !== null && !Array.isArray(name) ? (name as Record<string, Value>)[props.GROUP_NAME] : undefined) as string | undefined) ?? "<missing>";
  });
}

test("fieldcolumns_cases", () => {
  const report = new TestReport("fieldcolumns_cases", "fieldcolumns extraction / skip / reposition (src/fieldcolumns)");
  for (const testCase of yieldTestData()) {
    report.beginCase(newTestTitle(testCase.testTitle));
    let fields: ColumnFields;
    try {
      const options: ExtractOptions = {};
      fields = extract(sampleModel(), options);
    } catch (err) {
      report.failCase(`unexpected error: ${err instanceof Error ? err.message : String(err)}`);
      continue;
    }
    const op = testCase.op;
    if (op.kind === "extract") {
      const keys = columnKeys(fields, fields.current);
      if (keys.join("|") !== op.expectedKeys.join("|")) {
        report.failCase(`keys: expected ${JSON.stringify(op.expectedKeys)}, got ${JSON.stringify(keys)}`);
      }
      const names = columnNames(fields, fields.current);
      if (names.join("|") !== op.expectedNames.join("|")) {
        report.failCase(`names: expected ${JSON.stringify(op.expectedNames)}, got ${JSON.stringify(names)}`);
      }
    } else if (op.kind === "extractSkip") {
      fields.skip(op.skip, undefined);
      const keys = columnKeys(fields, fields.currentReadOrder());
      if (keys.join("|") !== op.expectedKeys.join("|")) {
        report.failCase(`keys: expected ${JSON.stringify(op.expectedKeys)}, got ${JSON.stringify(keys)}`);
      }
    } else {
      fields.reposition();
      fields.updateIndexInRepositioned();
      const keys = columnKeys(fields, fields.current);
      if (keys.join("|") !== op.expectedKeys.join("|")) {
        report.failCase(`keys: expected ${JSON.stringify(op.expectedKeys)}, got ${JSON.stringify(keys)}`);
      }
    }
  }
  report.finish();
});
