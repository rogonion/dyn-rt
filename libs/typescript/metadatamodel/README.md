# @dyn-rt/metadatamodel

dyn-rt metadatamodel module: the metadata model as an extensible plain JSON-like object (fields/groups with `<section>:<key>` props, always arrays down to the field props, `group:path_key` templates keeping `[*]`, single/array cardinality) with subdomains: core, fieldcolumns, database, iter, filter, flattener, unflattener, groupindex. Port of libs/rust/metadatamodel.

```ts
import { manipulation, metadatamodel } from "@dyn-rt/dyn-rt";
const core = metadatamodel.core;

const model = {
  [core.props.GROUP_PATH_KEY]: "$",
  [core.FIELDS_KEY]: [
    {
      ID: [
        {
          [core.props.GROUP_PATH_KEY]: "$['group:fields'][*].ID[*]",
          [core.props.GROUP_IS_PRIMARY_KEY]: true,
        },
      ],
    },
  ],
};

core.dataPath("$['group:fields'][*].ID[*]", [], "array"); // "$.ID[0]"

const rows = [{ ID: [101] }];
const flattener = new metadatamodel.flattener.Flattener(model);
flattener.flatten(manipulation.DynObject.new(rows));
flattener.result(); // [[[101]]]
```

Full examples (all subdomains, mixed cardinality, flatten/unflatten roundtrip, filter query conditions): <https://github.com/rogonion/dyn-rt/blob/main/docs/examples/typescript/metadatamodel.md> — plus the long-form model + query-condition JSON in `docs/architecture/descriptions/metadatamodel.md`.

Part of [dyn-rt](https://github.com/rogonion/dyn-rt). License: MIT (see LICENSE).
