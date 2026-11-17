# Description — the metadata model (`metadatamodel` module)

The metadata model describes the shape of source data (fields, groups, their
properties) so the dynamic layer (`path`, `manipulation`, `schema`) can work
with that data without knowing concrete types. It answers questions like
"which fields exist, in what order, with which column in which database
table, and how do I get/set the value of field X in this data".

Module: `libs/rust/metadatamodel` (crate `dyn-rt-metadatamodel`) and
`libs/typescript/metadatamodel` (npm `@dyn-rt/metadatamodel`, re-exported as
the `metadatamodel` namespace of the `@dyn-rt/dyn-rt` façade). One subdomain
per directory, each with `domain.ts` / `application.ts` / `index.ts` (Rust:
`domain.rs` / `application.rs`).

## The model is a JSON-like tree, not a concrete struct

The model is a `shared::Value` tree (Rust) / plain JS value (TS) — extensible
by construction: unknown properties are carried along untouched, so the model
can grow new `<section>:<key>` property vocabularies without code changes.

Structural rules:

- A **group** is an object. Its fields live under the structural key
  `group:fields`, which is **always an array whose first element** is the
  object mapping field-name → field/group.
- A **field entry** is likewise **always an array whose first element** is the
  props object. So a field's `group:path_key` template carries `[*]` all the
  way to the end, e.g. `$['group:fields'][*].Bio[*]`.
- Property keys are namespaced `<section>:<key>` — refinement of go's flat
  `FieldGroup*` / `Field*` / `Group*` / `Database*` names. Sections in use:
  `group:*`, `field:*`, `view:*`, `input:*`, `edit:*`, `query:*`, `extract:*`,
  `db:*` (see `src/core/domain.ts` for the full list).

## Comprehensive nested example

An Employee model: root fields `ID` (primary key) and `Skills` (expanded into
separate columns), plus a nested group `Profile` holding `Name`, `Age`, and a
double-nested group `Address` (`Street`, `City`, `ZipCode`), all with the
`db:*` mapping properties. Written in TypeScript (the house style — test data
and models are written in-code, and the model IS a plain JSON value):

```ts
import { metadatamodel } from "@dyn-rt/dyn-rt";

const { props, FIELDS_KEY } = metadatamodel.core;

const fld = (pathKey: string, name: string, dataType: string, extra: object = {}) => [
  { [props.GROUP_PATH_KEY]: pathKey, [props.GROUP_NAME]: name, [props.FIELD_DATA_TYPE]: dataType, ...extra },
];

const grp = (pathKey: string, name: string, fields: object, extra: object = {}) => [
  {
    [props.GROUP_PATH_KEY]: pathKey,
    [props.GROUP_NAME]: name,
    [FIELDS_KEY]: [fields],
    ...extra,
  },
];

const employeeModel = {
  [props.GROUP_PATH_KEY]: "$",
  [props.GROUP_NAME]: "Employee",
  [props.DB_TABLE_COLLECTION_UID]: "Employee",
  [props.DB_JOIN_DEPTH]: 0,
  [props.DB_TABLE_COLLECTION_NAME]: "Employee",
  [props.GROUP_READ_ORDER_OF_FIELDS]: ["ID", "Profile", "Skills"],
  [FIELDS_KEY]: [
    {
      ID: fld("$['group:fields'][*].ID[*]", "ID", "Number", {
        [props.GROUP_IS_PRIMARY_KEY]: true,
        [props.DB_TABLE_COLLECTION_UID]: "Employee",
        [props.DB_JOIN_DEPTH]: 0,
        [props.DB_TABLE_COLLECTION_NAME]: "Employee",
        [props.DB_COLUMN_NAME]: "ID",
      }),
      Profile: grp(
        "$['group:fields'][*].Profile[*]",
        "Profile",
        {
          Name: fld("$['group:fields'][*].Profile[*]['group:fields'][*].Name[*]", "Name", "Text", {
            [props.DB_TABLE_COLLECTION_UID]: "Profile",
            [props.DB_JOIN_DEPTH]: 1,
            [props.DB_TABLE_COLLECTION_NAME]: "Profile",
            [props.DB_COLUMN_NAME]: "Name",
          }),
          Age: fld("$['group:fields'][*].Profile[*]['group:fields'][*].Age[*]", "Age", "Number", {
            [props.DB_TABLE_COLLECTION_UID]: "Profile",
            [props.DB_JOIN_DEPTH]: 1,
            [props.DB_TABLE_COLLECTION_NAME]: "Profile",
            [props.DB_COLUMN_NAME]: "Age",
          }),
          Address: grp(
            "$['group:fields'][*].Profile[*]['group:fields'][*].Address[*]",
            "Address",
            {
              Street: fld("$['group:fields'][*].Profile[*]['group:fields'][*].Address[*]['group:fields'][*].Street[*]", "Street", "Text", { /* db:* of the Address collection */ }),
              City: fld("$['group:fields'][*].Profile[*]['group:fields'][*].Address[*]['group:fields'][*].City[*]", "City", "Text", { /* ... */ }),
              ZipCode: fld("$['group:fields'][*].Profile[*]['group:fields'][*].Address[*]['group:fields'][*].ZipCode[*]", "ZipCode", "Text", { /* ... */ }),
            },
            { [props.DB_TABLE_COLLECTION_UID]: "Address", [props.DB_JOIN_DEPTH]: 2, [props.DB_TABLE_COLLECTION_NAME]: "Address" },
          ),
        },
        { [props.DB_TABLE_COLLECTION_UID]: "Profile", [props.DB_JOIN_DEPTH]: 1, [props.DB_TABLE_COLLECTION_NAME]: "Profile" },
      ),
      Skills: fld("$['group:fields'][*].Skills[*]", "Skills", "Text", {
        [props.VIEW_VALUES_IN_SEPARATE_COLUMNS]: true,
        [props.VIEW_MAX_VALUES_IN_SEPARATE_COLUMNS]: 2,
        [props.VIEW_SEPARATE_COLUMNS_HEADER_FORMAT]: "skill [*]",
        [props.DB_COLUMN_NAME]: "Skills",
        [props.DB_TABLE_COLLECTION_UID]: "Employee",
        [props.DB_JOIN_DEPTH]: 0,
        [props.DB_TABLE_COLLECTION_NAME]: "Employee",
      }),
    },
  ],
};
```

Because the model is a plain value, the same document serializes to JSON
as-is and can itself be traversed with `path`/`manipulation`.

## Path templates: `group:path_key`

Each field/group carries a **templated JSONPath** that keeps `[*]` everywhere:

```
$['group:fields'][*].Profile[*]['group:fields'][*].Address[*]['group:fields'][*].City[*]
```

Core (`src/core/application.ts`) post-processes the template three ways:

- `schemaPath(template, indices)` — every `[*]` replaced in order by the given
  indices (missing default to 0): the path to the field/group **props inside
  the model**, e.g. indices `[0, 1]` →
  `$['group:fields'][0].Profile[1]['group:fields'][0].Address[0]...`.
- `dataPath(template, indices, cardinality)` — the structural
  `group:fields[*]` pairs are dropped and remaining `[*]` are indexed: the
  actual path to **data**, e.g. → `$.Profile[0].Address[1].City[0]`.
- `dataPathKeepWildcards(template)` — like `dataPath` but wildcards stay:
  `$.Profile[*].Address[*].City[*]` (used by the filter to iterate all values).
- `dataPathModelAware(template, model, indices, indexLastWildcard)` — walks
  the template together with the model so each field/group's **declared
  cardinality** decides whether its `[*]` is indexed (`array`, the default)
  or dropped (`single`).

## Cardinality: `single` vs `array`

`field:cardinality` / `group:cardinality` declare whether a field/group's
values are a list (`"array"`/`"list"`, the default) or a single value
(`"single"`/`"object"`). The rule is strict and committed: if `array` is
declared every value must be an array, if `single` every value must be
single — contradictions are a `CardinalityViolation` error, never a dynamic
exception. `dataPathModelAware` drops a `single` group's `[*]`; the
unflattener unwraps a `single` field's cell array on write (and errors on
more than one value unless the mode is `TakeFirst`).

## Subdomains

| Subdomain | What it does |
|---|---|
| `core` | Property getters, template post-processing (`schemaPath`/`dataPath`/`dataPathModelAware`/`dataPathKeepWildcards`), strict cardinality check |
| `fieldcolumns` | Extract the model's fields as flat 2D table columns (read order, skip, reposition, separate columns, `groupColumnIndexes`) |
| `database` | `getColumnFields` — the columns of one `db:table_collection` (by uid or join_depth+name, with skip/add); `FieldValue` get/set/delete of a column's values in source data through its `group:path_key` template |
| `iter` | Recursive `forEach` / `filter` / `map` walks over fields/groups in read order (callbacks get the props object) |
| `filter` | `DataFilter` — evaluate query conditions against a root sequence in source data, returning the excluded indexes (see below) |
| `flattener` | Nested collections → 2D table (cells are always arrays), branching per group-array element (Cartesian product); `writeToDestination` |
| `unflattener` | 2D table → nested collection written to a destination, merging rows into instances by primary-key signature; cardinality-aware writes |
| `groupindex` | The instance/signature machinery used by the unflattener: `GroupIndexNode`, `GroupCollection`, `Signature` |

## Filter query conditions

`filter.DataFilter` runs a query condition against every entry of a root
sequence in the source data (the root value at `rootJsonPathToValue` must be
a sequence) and returns the **indexes that did NOT pass** (go's `DataFilter.Filter`).
An empty query keeps everything. `silenceErrors = true` turns errors into
`false` instead of throwing.

Two condition shapes (go's `doc.go` example):

```jsonc
// 1. Logical operator: combine sub-conditions with And/Or, flip with Negate
{ "Type": "LogicalOperator", "LogicalOperator": "And" | "Or",
  "Negate": false, "Value": [ <query condition>, ... ] }

// 2. Field group: map schema path keys to per-field conditions
{ "Type": "FieldGroup", "Negate": false,
  "Value": { "<schema path key>": { "<condition>": { ... } } } }
```

Per-field conditions combine with AND within one field's object; the matched
values of a field (the path keeps its wildcards) combine with OR. Condition
names: `NoOfEntriesGreaterThan` / `NoOfEntriesLessThan` / `NoOfEntriesEqualTo`,
`GreaterThan` / `LessThan` / `EqualTo`, `BeginsWith` / `EndsWith` / `Contains`.
Condition data carries `AssumedFieldType` (`Text` | `Number` | `Timestamp` |
`Any`), plus `Value` or `Values` (array), `CaseInsensitive` (text),
`DateTimeFormat` (timestamp granularity: `yyyy`, `yyyy-mm`, `yyyy-mm-dd`,
`yyyy-mm-dd hh:mm`, `mm`, `hh:mm`).

### Examples (against the Employee model and a `EmployeeRow[]` source)

AND of a number range on `ID` and a text condition on `Name` — excludes
indexes `[0, 1, 3]` for products with IDs 0..3 where only "Product 2" matches:

```jsonc
{
  "Type": "LogicalOperator", "LogicalOperator": "And", "Negate": false,
  "Value": [
    { "Type": "FieldGroup", "Negate": false, "Value": {
      "$['group:fields'][*].ID[*]": {
        "GreaterThan": { "AssumedFieldType": "Number", "Value": 0 },
        "LessThan":    { "AssumedFieldType": "Number", "Value": 3 }
      } } } },
    { "Type": "FieldGroup", "Negate": false, "Value": {
      "$['group:fields'][*].Name[*]": {
        "EndsWith": { "AssumedFieldType": "Text", "Value": "2" }
      } } } }
  ]
}
```

OR over a nested group's entry count and a nested field:

```jsonc
{ "Type": "LogicalOperator", "LogicalOperator": "Or", "Negate": false,
  "Value": [
    { "Type": "FieldGroup", "Negate": false, "Value": {
      "$['group:fields'][*].Address[*]": {
        "NoOfEntriesGreaterThan": { "Value": 1 } } } },
    { "Type": "FieldGroup", "Negate": false, "Value": {
      "$['group:fields'][*].Address[*]['group:fields'][*].City[*]": {
        "BeginsWith": { "AssumedFieldType": "Text", "Value": "Streete" } } } }
  ] }
```

Negated field condition with the root focused on one entry's group array
(`rootJsonPathKey` / `rootJsonPathToValue`) — against `$[2].Address` of a
`UserProfileRow[]`, excludes `[1]` (only the second address entry, whose City
begins with "City 4", fails the negated condition):

```ts
const filter = new metadatamodel.filter.DataFilter(
  manipulation.DynObject.new(userProfiles),
  userProfileModel,
);
const excluded = filter.filter(
  { Type: "FieldGroup", Negate: true, Value: {
    "$['group:fields'][*].Address[*]['group:fields'][*].City[*]": {
      BeginsWith: { AssumedFieldType: "Text", Value: "City 4" } } } },
  "$['group:fields'][*].Address[*]", // rootJsonPathKey
  "$[2].Address",                    // rootJsonPathToValue
);
// => [1]
```

Timestamp condition compared at `yyyy-mm-dd` granularity:

```jsonc
{ "Type": "FieldGroup", "Negate": false, "Value": {
  "$['group:fields'][*].Created[*]": {
    "GreaterThan": {
      "AssumedFieldType": "Timestamp",
      "DateTimeFormat": "yyyy-mm-dd",
      "Value": "2024-01-01T00:00:00Z"
    }
  } } }
```

Text equality against multiple values, case-insensitive, negated at the field
level:

```jsonc
{ "Type": "FieldGroup", "Negate": true, "Value": {
  "$['group:fields'][*].Name[*]": {
    "EqualTo": {
      "AssumedFieldType": "Text",
      "CaseInsensitive": true,
      "Values": ["product 1", "PRODUCT 3"]
    }
  } } }
```

## Flatten / unflatten

`flattener.Flattener` converts a nested collection into a 2D table where every
cell is an array (empty cells are empty arrays); nested group arrays branch
into a Cartesian product of rows. `unflattener.Unflattener` converts the table
back into a nested collection in a destination (attach a schema to the
destination's `DynObject` to guide creation/coercion), merging rows that share
a primary-key signature into one instance. Flatten → unflatten round-trips the
original collection (covered by a test in `tests/unflattener.test.ts`).
