// Shared test fixtures for the schema suites — the TS port of
// libs/rust/test-support/src/fixtures.rs (which mirrors go-json's
// schema/misc_test.go). Plain classes stand in for Rust structs; the
// capitalized field names keep parity with the go fixtures. Schema builders
// mirror the *Schema() funcs of misc_test.go.

import {
  any,
  enumNode,
  indirectionNode,
  jsonScalar,
  mapNode,
  nilable,
  scalar,
  schemaSet,
  sequenceNode,
  classNode,
  type Schema,
  type SchemaNode,
} from "../src/index.js";

// ---------------------------------------------------------------------------
// test types (mirrors misc_test.go)
// ---------------------------------------------------------------------------

export class User {
  ID = 0;
  Name = "";
  Email = "";
  constructor(init?: Partial<User>) {
    Object.assign(this, init);
  }
}

export class Product {
  ID = 0;
  Name = "";
  Price = 0;
  constructor(init?: Partial<Product>) {
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

export class Employee {
  ID = 0;
  Profile = new UserProfile();
  Skills: string[] = [];
  ProjectHours: Record<string, number> = {};
  constructor(init?: Partial<Employee>) {
    Object.assign(this, init);
  }
}

export class Company {
  Name = "";
  Employees: User[] = [];
  constructor(init?: Partial<Company>) {
    Object.assign(this, init);
  }
}

export class NestedItem {
  ID = 0;
  MapData: Record<string, unknown> = {};
  ListData: unknown[] = [];
  constructor(init?: Partial<NestedItem>) {
    Object.assign(this, init);
  }
}

export class Circle {
  Radius = 0;
  constructor(init?: Partial<Circle>) {
    Object.assign(this, init);
  }
}

export class Square {
  Side = 0;
  constructor(init?: Partial<Square>) {
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

/** go's `Shape` interface as a TS interface: every shape implements it. A
 * conversion target typed as `Shape` still participates because TS is
 * structural. */
export interface Shape {
  isShape(): boolean;
}

// ---------------------------------------------------------------------------
// schema constructors (mirror the *Schema() funcs of misc_test.go)
// ---------------------------------------------------------------------------

export const stringNode = (): SchemaNode => scalar("string");
export const intNode = (): SchemaNode => scalar("i64");
export const floatNode = (): SchemaNode => scalar("f64");
export const boolNode = (): SchemaNode => scalar("bool");
export const anyNode = (): SchemaNode => any();
export const jsonStringNode = (): SchemaNode => jsonScalar("string");

export function userSchema(): SchemaNode {
  return classNode("User", [
    ["ID", intNode()],
    ["Name", stringNode()],
    ["Email", stringNode()],
  ]);
}

export function productSchema(): SchemaNode {
  return classNode("Product", [
    ["ID", intNode()],
    ["Name", stringNode()],
    ["Price", floatNode()],
  ]);
}

export function listOfProductsSchema(): SchemaNode {
  return sequenceNode(productSchema());
}

export function addressSchema(): SchemaNode {
  return classNode("Address", [
    ["Street", stringNode()],
    ["City", stringNode()],
    // go: ZipCode *string -> TS: string | null, modeled as a nilable enum of
    // Some(string) / None (the Option shape)
    ["ZipCode", nilable(enumNode([["Some", stringNode()], ["None", undefined]]))],
  ]);
}

export function userProfileSchema(): SchemaNode {
  return classNode("UserProfile", [
    ["Name", stringNode()],
    ["Age", intNode()],
    ["Address", addressSchema()],
  ]);
}

export function employeeSchema(): SchemaNode {
  return classNode("Employee", [
    ["ID", intNode()],
    ["Profile", userProfileSchema()],
    ["Skills", sequenceNode(stringNode())],
    ["ProjectHours", mapNode(stringNode(), intNode())],
  ]);
}

export function companySchema(): SchemaNode {
  return classNode("Company", [
    ["Name", stringNode()],
    // the indirection mirrors go's []*User (pointer inside a sequence); in TS
    // values are already references, so it converts straight through
    ["Employees", sequenceNode(indirectionNode(userSchema()))],
  ]);
}

export function nestedItemSchema(): SchemaNode {
  return classNode("NestedItem", [
    ["ID", intNode()],
    ["MapData", mapNode(jsonStringNode(), anyNode())],
    ["ListData", sequenceNode(anyNode())],
  ]);
}

export function listOfNestedItemSchema(): SchemaNode {
  return sequenceNode(nestedItemSchema());
}

/** go's JsonMapSchema: map[string]any */
export function jsonMapSchema(): SchemaNode {
  return mapNode(jsonStringNode(), anyNode());
}

export function circleSchema(): SchemaNode {
  return classNode("Circle", [["Radius", floatNode()]]);
}

export function squareSchema(): SchemaNode {
  return classNode("Square", [["Side", floatNode()]]);
}

/** go's ShapeSchema: a set of Circle/Square (default key "default" is
 * intentionally absent from the nodes, as in the go fixture) */
export function shapeSchema(): Schema {
  return schemaSet("default", [
    ["Circle", circleSchema()],
    ["Square", squareSchema()],
  ]);
}

export function listOfShapesSchema(): SchemaNode {
  return sequenceNode(shapeSchema());
}

/** go's MapUserSchema: map[int]*User — JS object keys are strings, so the int
 * key schema coerces the key string to a number and back */
export function mapUserSchema(): SchemaNode {
  return mapNode(intNode(), indirectionNode(userSchema()));
}

export function userWithAddressSchema(): SchemaNode {
  return classNode("UserWithAddress", [
    ["Name", stringNode()],
    ["Address", indirectionNode(addressSchema())],
  ]);
}

// ---------------------------------------------------------------------------
// sample data
// ---------------------------------------------------------------------------

export function sampleUser(): User {
  return new User({ ID: 1, Name: "amy", Email: "amy@example.com" });
}

export function userValue(): Record<string, unknown> {
  return { ID: 1, Name: "amy", Email: "amy@example.com" };
}

export function sampleAddress(): Address {
  return new Address({ Street: "1 Main St", City: "Springfield", ZipCode: "12345" });
}

export function sampleUserProfile(): UserProfile {
  return new UserProfile({ Name: "amy", Age: 30, Address: sampleAddress() });
}

export function sampleProduct(): Product {
  return new Product({ ID: 7, Name: "widget", Price: 9.99 });
}

export function sampleEmployee(): Employee {
  return new Employee({
    ID: 3,
    Profile: sampleUserProfile(),
    Skills: ["rust", "go"],
    ProjectHours: { core: 40 },
  });
}

export function sampleCompany(): Company {
  return new Company({ Name: "acme", Employees: [sampleUser()] });
}

export function sampleNestedItem(): NestedItem {
  return new NestedItem({ ID: 1, MapData: { k: "v" }, ListData: [1, "x"] });
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
    const record = value as Record<string, unknown>;
    const entries = Object.keys(record)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${stableStringify(record[key])}`);
    return `{${entries.join(",")}}`;
  }
  return JSON.stringify(value) ?? String(value);
}
