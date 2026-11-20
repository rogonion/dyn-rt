// Application-layer suite for reflect (ADR 0004/0009): sequence/map/class/
// scalar accessors plus reflectClone / reflectZero / reflectApply
// (src/application.ts).

import { test } from "vitest";
import { TestReport, newTestTitle } from "@dyn-rt/test-support";
import {
  asMap,
  asSequence,
  asClass,
  mapContainsKey,
  mapDelete,
  mapGet,
  mapKeys,
  mapLength,
  mapSet,
  reflectApply,
  reflectClone,
  reflectZero,
  scalarBigint,
  scalarBool,
  scalarNumber,
  scalarString,
  seqDeleteIndex,
  seqGet,
  seqLength,
  seqPush,
  seqSet,
  classFieldNames,
  classGet,
  classSet,
} from "../src/index.js";

class Address {
  constructor(
    public Street: string,
    public City: string,
  ) {}
}

class User {
  constructor(
    public ID: number,
    public Name: string,
    public Address: Address,
    public Tags: string[],
  ) {}
}

interface AccessorCase {
  testTitle?: string;
  run: () => unknown;
  expected: unknown;
}

function eq(a: unknown, b: unknown): boolean {
  const replacer = (_key: string, v: unknown) => (typeof v === "bigint" ? `${v}n` : v);
  return JSON.stringify(a, replacer) === JSON.stringify(b, replacer);
}

const accessorCases: AccessorCase[] = [
  {
    testTitle: "sequence accessors: length/get/set/push/delete",
    run: () => {
      const seq: unknown[] = [1, 2];
      const out: unknown[] = [seqLength(seq), seqGet(seq, 0), seqGet(seq, 9)];
      seqSet(seq, 3, "x"); // grows
      seqPush(seq, "y");
      seqDeleteIndex(seq, 0);
      out.push(seq);
      return out;
    },
    expected: [2, 1, undefined, [2, undefined, "x", "y"]],
  },
  {
    testTitle: "map accessors: get/set/delete/contains/keys/length",
    run: () => {
      const map: Record<string, unknown> = { a: 1 };
      mapSet(map, "b", 2);
      const out: unknown[] = [mapGet(map, "a"), mapContainsKey(map, "zzz"), mapKeys(map), mapLength(map)];
      out.push(mapDelete(map, "a"), mapDelete(map, "a"));
      out.push(map);
      return out;
    },
    expected: [1, false, ["a", "b"], 2, true, false, { b: 2 }],
  },
  {
    testTitle: "class accessors over a class instance",
    run: () => {
      const user = new User(1, "Alice", new Address("Main St", "Nairobi"), ["x"]);
      const names = classFieldNames(user);
      classSet(user, "Name", "Bob");
      return [names, classGet(user, "Name"), classGet(user, "Address") instanceof Address];
    },
    expected: [["ID", "Name", "Address", "Tags"], "Bob", true],
  },
  {
    testTitle: "kind-mismatch accessors return undefined",
    run: () => [asSequence({}), asMap([1]), asClass({ a: 1 })],
    expected: [undefined, undefined, undefined],
  },
  {
    testTitle: "scalar accessors",
    run: () => [scalarNumber(3.5), scalarNumber("x"), scalarBigint(7n), scalarBool(false), scalarString("s"), scalarString(1)],
    expected: [3.5, undefined, 7n, false, "s", undefined],
  },
  {
    testTitle: "reflectClone deep-clones and preserves class prototype",
    run: () => {
      const user = new User(1, "Alice", new Address("Main St", "Nairobi"), ["x"]);
      const copy = reflectClone(user);
      copy.Tags.push("y");
      copy.Address.City = "Mombasa";
      return [copy instanceof User, copy.Address instanceof Address, user.Tags, user.Address.City, eq(copy, new User(1, "Alice", new Address("Main St", "Mombasa"), ["x", "y"]))];
    },
    expected: [true, true, ["x"], "Nairobi", true],
  },
  {
    testTitle: "reflectClone handles Date and arrays",
    run: () => {
      const date = new Date(1000);
      const copy = reflectClone({ d: date, arr: [1, [2]] });
      return [copy.d instanceof Date, copy.d.getTime(), copy.arr];
    },
    expected: [true, 1000, [1, [2]]],
  },
  {
    testTitle: "reflectZero by shape",
    run: () => [
      reflectZero(5),
      reflectZero(5n),
      reflectZero(true),
      reflectZero("s"),
      reflectZero(null),
      reflectZero([1, 2]),
      reflectZero({ a: 1 }),
      reflectZero(new User(1, "A", new Address("s", "c"), ["t"])),
      reflectZero(new Date(0)),
    ],
    expected: [0, 0n, false, "", null, [], {}, { ID: 0, Name: "", Address: { Street: "", City: "" }, Tags: [] }, undefined],
  },
  {
    testTitle: "reflectApply replaces map contents",
    run: () => {
      const target: Record<string, unknown> = { a: 1, b: 2 };
      reflectApply(target, { b: 3, c: 4 });
      return target;
    },
    expected: { b: 3, c: 4 },
  },
];

test("accessor cases (src/application.ts)", () => {
  const report = new TestReport("accessor_cases", "sequence/map/class/scalar accessors + clone/zero/apply (src/application.ts)");
  for (const testCase of accessorCases) {
    report.beginCase(newTestTitle(testCase.testTitle));
    const got = testCase.run();
    if (!eq(got, testCase.expected)) {
      report.failCase(`expected ${JSON.stringify(testCase.expected)}, got ${JSON.stringify(got)}`);
    }
  }
  report.finish();
});
