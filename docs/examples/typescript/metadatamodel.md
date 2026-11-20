# Examples — `metadatamodel` (TypeScript)

Module package: `@dyn-rt/metadatamodel` (`libs/typescript/metadatamodel`),
facade namespace `metadatamodel` from `@dyn-rt/dyn-rt`.

Subdomains: `core`, `fieldcolumns`, `database`, `iter`, `filter`,
`flattener`, `unflattener`, `groupindex`.

The metadata model is an extensible **plain JSON-like object** (not a fixed
class). Field/group properties use `<section>:<key>` names (`core.props`).
Structural keys are always arrays: `group:fields` is an array whose `[0]` is
the fields object; each field entry is an array whose `[0]` is the field's
props object. `group:path_key` templates keep their `[*]` placeholders;
`core.dataPath` post-processes a template into the actual path to data.

Cardinality: `"array"` (default; every value is a list) or `"single"`
(every value is a single object/scalar — a committed choice; violations are
errors, never dynamic exceptions).

See also
[docs/architecture/descriptions/metadatamodel.md](../../architecture/descriptions/metadatamodel.md)
for a comprehensive nested model (groups, `db:*` column mapping, separate
columns) and the full JSON structure of filter query conditions.

## The model (core vocabulary)

```ts
import { metadatamodel } from "@dyn-rt/dyn-rt";
const core = metadatamodel.core;

const model = {
  [core.props.GROUP_PATH_KEY]: "$",
  [core.FIELDS_KEY]: [
    {
      ID: [
        {
          [core.props.GROUP_PATH_KEY]: "$['group:fields'][*].ID[*]",
          [core.props.GROUP_IS_PRIMARY_KEY]: true,
          [core.props.DB_TABLE_COLLECTION_UID]: "users",
          [core.props.DB_COLUMN_NAME]: "id",
        },
      ],
      Name: [
        {
          [core.props.GROUP_PATH_KEY]: "$['group:fields'][*].Name[*]",
          [core.props.DB_TABLE_COLLECTION_UID]: "users",
          [core.props.DB_COLUMN_NAME]: "name",
        },
      ],
    },
  ],
};

core.pathKeyOf(model);            // "$"
Object.keys(core.fieldsOf(model)); // ["ID", "Name"]
core.cardinalityOf(model);         // "array" (default)
```

## core — path templates: dataPath / schemaPath

```ts
core.dataPath("$['group:fields'][*].ID[*]", [], "array");  // "$.ID[0]"
core.dataPath("$['group:fields'][*].ID[*]", [], "single"); // "$.ID" (trailing [*] trimmed)
core.schemaPath("$['group:fields'][*].ID[*]", [0]);        // "$['group:fields'][0].ID[0]"
```

## fieldcolumns / database

```ts
metadatamodel.fieldcolumns.extract(model).currentReadOrder(); // 2 columns

const columnFields = metadatamodel.database.getColumnFields(model, {
  tableCollectionUid: "users",
});
columnFields.readOrder; // ["id", "name"]

// read/write values by db:column_name instead of by path:
const rows = [{ ID: [1], Name: ["Alice"] }];
const fv = new metadatamodel.database.FieldValue(
  manipulation.DynObject.new(rows),
  columnFields,
);
fv.get("id", "", [0]);                    // values at the model-resolved path
fv.set("name", "Alicia", "", [0]);        // schema-guided set
```

## flattener / unflattener — nested data <-> 2D table

```ts
import { manipulation, metadatamodel, schema } from "@dyn-rt/dyn-rt";

const rows = [{ ID: [1], Name: ["Alice"] }];

const flattener = new metadatamodel.flattener.Flattener(model);
flattener.flatten(manipulation.DynObject.new(rows));
flattener.result(); // [[[1], ["Alice"]]]

// back: destination is an array with a schema attached (guides creation)
const dest: unknown[] = [];
const unflattener = new metadatamodel.unflattener.Unflattener(
  model,
  manipulation.DynObject.new(dest).withSchema(
    schema.sequenceNode(
      schema.classNode("Row", [
        ["ID", schema.sequenceNode(schema.scalar("number"))],
        ["Name", schema.sequenceNode(schema.scalar("string"))],
      ]),
    ),
  ),
);
unflattener.unflatten(flattener.result());
manipulation.areEqual(dest, rows); // true
```

### Mixed cardinality (array + single in one model)

```ts
const mixedModel = {
  [core.props.GROUP_PATH_KEY]: "$",
  [core.FIELDS_KEY]: [
    {
      ID: [
        {
          [core.props.GROUP_PATH_KEY]: "$['group:fields'][*].ID[*]",
          [core.props.GROUP_IS_PRIMARY_KEY]: true,
        },
      ],
      Profile: [
        {
          [core.props.GROUP_PATH_KEY]: "$['group:fields'][*].Profile",
          [core.props.GROUP_CARDINALITY]: "single",
          [core.FIELDS_KEY]: [
            {
              Age: [
                {
                  [core.props.GROUP_PATH_KEY]:
                    "$['group:fields'][*].Profile['group:fields'][*].Age",
                  [core.props.FIELD_CARDINALITY]: "single",
                },
              ],
            },
          ],
        },
      ],
    },
  ],
};

core.dataPath("$['group:fields'][*].Profile['group:fields'][*].Age", [], "single");
// "$.Profile.Age"   (vs "$.ID[0]" for the array-cardinality field)
```

A row `{ ID: [1], Profile: { Age: 30 } }` flattens to a table and unflattens
back to an equal value (verified roundtrip with mixed cardinality).

## filter — query conditions over model-described data

`DataFilter.filter(queryCondition)` returns the indexes that did **not** pass.

```ts
const query = {
  [metadatamodel.filter.query.TYPE]: metadatamodel.filter.query.TYPE_FIELD_GROUP,
  [metadatamodel.filter.query.NEGATE]: false,
  [metadatamodel.filter.query.VALUE]: {
    "$['group:fields'][*].Name[*]": {
      [metadatamodel.filter.condition.EQUAL_TO]: {
        [metadatamodel.filter.conditionProps.ASSUMED_FIELD_TYPE]:
          metadatamodel.filter.fieldType.TEXT,
        [metadatamodel.filter.conditionProps.VALUE]: "Alice",
      },
    },
  },
};

const rows2 = [
  { ID: [1], Name: ["Alice"] },
  { ID: [2], Name: ["Bob"] },
];
const df = new metadatamodel.filter.DataFilter(manipulation.DynObject.new(rows2), model);
df.filter(query); // [1] — indexes where Name != "Alice"
```

Conditions: `EqualTo`, `GreaterThan`, `LessThan`, `BeginsWith`, `EndsWith`,
`Contains`, `NoOfEntries*`; combined with `And`/`Or` logical operators and
`Negate`. Full JSON examples: the metadatamodel description doc linked above.

## iter — walk the model itself

```ts
const keys: string[] = [];
metadatamodel.iter.forEach(model, (props) => {
  const k = (props as Record<string, unknown>)[core.props.GROUP_PATH_KEY];
  if (typeof k === "string") keys.push(k);
  return metadatamodel.iter.Visit.CONTINUE; // or TERMINATE / skipFields
});
// keys == ["$['group:fields'][*].ID[*]", "$['group:fields'][*].Name[*]"]
```

`iter.filter(model, callback)` prunes fields/groups in place; `iter.map`
transforms props.

## groupindex — stable instance indexes for a flattened table

```ts
const sig = new metadatamodel.groupindex.Signature(); // joins with '|'
sig.generateSignature([1, "Alice"], [0]); // "1"

const node = new metadatamodel.groupindex.GroupIndexNode();
const collection = node.getOrCreateGroup("Profile");
collection.getOrCreateInstance("1", "$")[0]; // 0 (new)
collection.getOrCreateInstance("1", "$")[0]; // 0 (same instance again)
```
