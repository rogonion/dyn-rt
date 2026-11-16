// Shared test fixtures for the manipulation suites (ADR 0004/0009) — the TS
// port of libs/rust/manipulation/tests/common/mod.rs + the test-support
// fixtures: plain classes stand in for Rust structs, plain objects for
// map[string]any / Value objects, arrays for slices/tuples. Capitalized field
// names keep parity with the go fixtures.

import { mapNode, nilable, scalar, structNode, type SchemaNode } from "@dyn-rt/schema";

const stringNode = (): SchemaNode => scalar("string");

// ---------------------------------------------------------------------------
// test types (mirrors misc_test.go / object/misc_test.go)
// ---------------------------------------------------------------------------

export class User {
  ID = 0;
  Name = "";
  Email = "";
  constructor(init?: Partial<User>) {
    Object.assign(this, init);
  }
}

export class Address {
  Street = "";
  City = "";
  ZipCode: string | null = null;
  constructor(init?: Partial<Address>) {
    Object.assign(this, init);
  }
}

export class UserProfile {
  Name = "";
  Age = 0;
  Address = new Address();
  constructor(init?: Partial<UserProfile>) {
    Object.assign(this, init);
  }
}

export class ComplexDataItem {
  Name = "";
  Value = 0;
  constructor(init?: Partial<ComplexDataItem>) {
    Object.assign(this, init);
  }
}

export class ComplexData {
  ID = 0;
  Details: Record<string, unknown> = {};
  Items: ComplexDataItem[] = [];
  User = new User();
  constructor(init?: Partial<ComplexData>) {
    Object.assign(this, init);
  }
}

export class UserWithAddress {
  Name = "";
  Address = new Address();
  constructor(init?: Partial<UserWithAddress>) {
    Object.assign(this, init);
  }
}

// ---------------------------------------------------------------------------
// schemas (used by the set suite for coercion/creation)
// ---------------------------------------------------------------------------

export function userProfileSchema(): SchemaNode {
  return structNode("UserProfile", [
    ["Name", stringNode()],
    ["Age", scalar("i64")],
    [
      "Address",
      structNode("Address", [
        ["Street", stringNode()],
        ["City", stringNode()],
        // go's ZipCode *string -> TS string | null, modeled as a nilable scalar
        ["ZipCode", nilable(stringNode())],
      ]),
    ],
  ]);
}

/** map[string]int — used to show schema-driven coercion and conversion errors. */
export function stringIntMapSchema(): SchemaNode {
  return mapNode(stringNode(), scalar("i64"));
}

// ---------------------------------------------------------------------------
// sample data
// ---------------------------------------------------------------------------

export function sampleUser(): User {
  return new User({ ID: 1, Name: "amy", Email: "amy@example.com" });
}

export function sampleUserProfile(): UserProfile {
  return new UserProfile({
    Name: "amy",
    Age: 30,
    Address: new Address({ Street: "1 Main St", City: "Springfield", ZipCode: "12345" }),
  });
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/** Order-insensitive deep equality for plain data + class instances (own
 * enumerable props), with bigint support. */
export function eq(a: unknown, b: unknown): boolean {
  return stableStringify(a) === stableStringify(b);
}

function stableStringify(value: unknown): string {
  if (typeof value === "bigint") return `${value}n`;
  if (typeof value === "undefined") return "undefined";
  if (Array.isArray(value)) return `[${value.map(stableStringify).join(",")}]`;
  if (value !== null && typeof value === "object") {
    if (value instanceof Date) return `Date(${value.getTime()})`;
    const record = value as Record<string, unknown>;
    const entries = Object.keys(record)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${stableStringify(record[key])}`);
    return `{${entries.join(",")}}`;
  }
  return JSON.stringify(value) ?? String(value);
}

/** Renders a value for failure messages. */
export function render(value: unknown): string {
  return stableStringify(value);
}
