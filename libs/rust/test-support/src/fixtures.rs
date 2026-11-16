//! Shared test fixtures for dyn-rt suites: test types and schema constructors
//! mirroring go-json's schema/misc_test.go and object/misc_test.go shapes.
//! Lives here (dev-only, like the harness) so 2+ modules' test suites can
//! share them — e.g. dyn-rt-schema and dyn-rt-manipulation (the
//! manipulation suites reuse the same User/Address/... shapes the go
//! object tests use).

#![allow(dead_code, unused_macros, unused_imports)]

use std::collections::HashMap;

use dyn_rt_reflect::{Reflect, ReflectEnum, ReflectStruct};
use dyn_rt_schema::{Schema, SchemaNode, SchemaSet};
use dyn_rt_shared::Value;

// ---------------------------------------------------------------------------
// test types (mirrors misc_test.go)
// ---------------------------------------------------------------------------

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct User {
    #[reflect(rename = "ID")]
    pub id: i64,
    #[reflect(rename = "Name")]
    pub name: String,
    #[reflect(rename = "Email")]
    pub email: String,
}

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct Product {
    #[reflect(rename = "ID")]
    pub id: i64,
    #[reflect(rename = "Name")]
    pub name: String,
    #[reflect(rename = "Price")]
    pub price: f64,
}

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct Address {
    #[reflect(rename = "Street")]
    pub street: String,
    #[reflect(rename = "City")]
    pub city: String,
    #[reflect(rename = "ZipCode")]
    pub zip_code: Option<String>,
}

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct UserProfile {
    #[reflect(rename = "Name")]
    pub name: String,
    #[reflect(rename = "Age")]
    pub age: i64,
    #[reflect(rename = "Address")]
    pub address: Address,
}

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct UserProfile2 {
    #[reflect(rename = "Name")]
    pub name: String,
    #[reflect(rename = "Age")]
    pub age: i64,
    #[reflect(rename = "Country")]
    pub country: String,
    #[reflect(rename = "Occupation")]
    pub occupation: String,
}

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct Employee {
    #[reflect(rename = "ID")]
    pub id: i64,
    #[reflect(rename = "Profile")]
    pub profile: UserProfile,
    #[reflect(rename = "Skills")]
    pub skills: Vec<String>,
    #[reflect(rename = "ProjectHours")]
    pub project_hours: HashMap<String, i64>,
}

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct Company {
    #[reflect(rename = "Name")]
    pub name: String,
    #[reflect(rename = "Employees")]
    // Box<User> mirrors go's []*User (indirection inside a sequence)
    #[allow(clippy::vec_box)]
    pub employees: Vec<Box<User>>,
}

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct NestedItem {
    #[reflect(rename = "ID")]
    pub id: i64,
    #[reflect(rename = "MapData")]
    pub map_data: HashMap<String, Value>,
    #[reflect(rename = "ListData")]
    pub list_data: Vec<Value>,
}

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct Circle {
    #[reflect(rename = "Radius")]
    pub radius: f64,
}

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct Square {
    #[reflect(rename = "Side")]
    pub side: f64,
}

#[derive(ReflectEnum, Clone, Debug, PartialEq)]
pub enum ShapeEnum {
    Circle(Circle),
    Square(Square),
}

/// go's `Shape` interface as a Rust trait: every shape implements it. With
/// `Reflect` as a supertrait, `Box<dyn Shape>` / `&dyn Shape` upcast to
/// `Box<dyn Reflect>` / `&dyn Reflect` (trait upcasting), so a conversion
/// target can be written against `dyn Shape` and still participate in
/// reflection.
pub trait Shape: dyn_rt_reflect::Reflect {
    fn is_shape(&self) -> bool;
}

impl Shape for Circle {
    fn is_shape(&self) -> bool {
        true
    }
}

impl Shape for Square {
    fn is_shape(&self) -> bool {
        true
    }
}

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct UserWithAddress {
    #[reflect(rename = "Name")]
    pub name: String,
    #[reflect(rename = "Address")]
    pub address: Box<Address>,
}

// ---------------------------------------------------------------------------
// schema constructors (mirror the *Schema() funcs of misc_test.go)
// ---------------------------------------------------------------------------

pub fn string_node() -> SchemaNode {
    SchemaNode::scalar::<String>()
}

pub fn int_node() -> SchemaNode {
    SchemaNode::scalar::<i64>()
}

pub fn float_node() -> SchemaNode {
    SchemaNode::scalar::<f64>()
}

pub fn any_node() -> SchemaNode {
    SchemaNode::any()
}

pub fn json_string_node() -> SchemaNode {
    SchemaNode::json_scalar("string")
}

pub fn user_schema() -> SchemaNode {
    SchemaNode::struct_node::<User>(vec![
        ("ID", Schema::Node(int_node())),
        ("Name", Schema::Node(string_node())),
        ("Email", Schema::Node(string_node())),
    ])
}

pub fn product_schema() -> SchemaNode {
    SchemaNode::struct_node::<Product>(vec![
        ("ID", Schema::Node(int_node())),
        ("Name", Schema::Node(string_node())),
        ("Price", Schema::Node(float_node())),
    ])
}

pub fn list_of_products_schema() -> SchemaNode {
    SchemaNode::sequence_node::<Vec<Product>>(Schema::Node(product_schema()))
}

pub fn address_schema() -> SchemaNode {
    SchemaNode::struct_node::<Address>(vec![
        ("Street", Schema::Node(string_node())),
        ("City", Schema::Node(string_node())),
        // go: ZipCode *string -> rust: Option<String> (enum with Some/None)
        (
            "ZipCode",
            Schema::Node(
                SchemaNode::enum_node::<Option<String>>(vec![
                    ("Some", Some(Schema::Node(string_node()))),
                    ("None", None),
                ])
                .nilable(),
            ),
        ),
    ])
}

pub fn user_profile_schema() -> SchemaNode {
    SchemaNode::struct_node::<UserProfile>(vec![
        ("Name", Schema::Node(string_node())),
        ("Age", Schema::Node(int_node())),
        ("Address", Schema::Node(address_schema())),
    ])
}

pub fn user_profile2_schema() -> SchemaNode {
    SchemaNode::struct_node::<UserProfile2>(vec![
        ("Name", Schema::Node(string_node())),
        ("Age", Schema::Node(int_node())),
        ("Country", Schema::Node(string_node())),
        ("Occupation", Schema::Node(string_node())),
    ])
}

pub fn employee_schema() -> SchemaNode {
    SchemaNode::struct_node::<Employee>(vec![
        ("ID", Schema::Node(int_node())),
        ("Profile", Schema::Node(user_profile_schema())),
        (
            "Skills",
            Schema::Node(SchemaNode::sequence_node::<Vec<String>>(Schema::Node(
                string_node(),
            ))),
        ),
        (
            "ProjectHours",
            Schema::Node(SchemaNode::map_node::<HashMap<String, i64>>(
                Schema::Node(string_node()),
                Schema::Node(int_node()),
            )),
        ),
    ])
}

pub fn company_schema() -> SchemaNode {
    SchemaNode::struct_node::<Company>(vec![
        ("Name", Schema::Node(string_node())),
        (
            "Employees",
            Schema::Node(SchemaNode::sequence_node::<Vec<Box<User>>>(Schema::Node(
                SchemaNode::indirection_node::<Box<User>>(Schema::Node(user_schema())),
            ))),
        ),
    ])
}

pub fn nested_item_schema() -> SchemaNode {
    SchemaNode::struct_node::<NestedItem>(vec![
        ("ID", Schema::Node(int_node())),
        (
            "MapData",
            Schema::Node(SchemaNode::map_node::<HashMap<String, Value>>(
                Schema::Node(json_string_node()),
                Schema::Node(any_node()),
            )),
        ),
        (
            "ListData",
            Schema::Node(SchemaNode::sequence_node::<Vec<Value>>(Schema::Node(
                any_node(),
            ))),
        ),
    ])
}

pub fn list_of_nested_item_schema() -> SchemaNode {
    SchemaNode::sequence_node::<Vec<NestedItem>>(Schema::Node(nested_item_schema()))
}

/// go's JsonMapSchema: map[string]interface{}
pub fn json_map_schema() -> SchemaNode {
    SchemaNode::map_node::<Value>(Schema::Node(json_string_node()), Schema::Node(any_node()))
}

pub fn circle_schema() -> SchemaNode {
    SchemaNode::struct_node::<Circle>(vec![("Radius", Schema::Node(float_node()))])
}

pub fn square_schema() -> SchemaNode {
    SchemaNode::struct_node::<Square>(vec![("Side", Schema::Node(float_node()))])
}

/// go's ShapeSchema: a DynamicSchema of Circle/Square (default key "default"
/// is intentionally absent from the nodes, as in the go fixture)
pub fn shape_schema() -> Schema {
    Schema::Set(SchemaSet::new(
        Some("default"),
        vec![("Circle", circle_schema()), ("Square", square_schema())],
    ))
}

pub fn list_of_shapes_schema() -> SchemaNode {
    SchemaNode::sequence_node::<Vec<ShapeEnum>>(shape_schema())
}

/// go's MapUserSchema: map[int]*User
pub fn map_user_schema() -> SchemaNode {
    SchemaNode::map_node::<HashMap<i64, Box<User>>>(
        Schema::Node(int_node()),
        Schema::Node(SchemaNode::indirection_node::<Box<User>>(Schema::Node(
            user_schema(),
        ))),
    )
}

pub fn user_with_address_schema() -> SchemaNode {
    SchemaNode::struct_node::<UserWithAddress>(vec![
        ("Name", Schema::Node(string_node())),
        (
            "Address",
            Schema::Node(SchemaNode::indirection_node::<Box<Address>>(Schema::Node(
                address_schema(),
            ))),
        ),
    ])
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

pub fn sample_user() -> User {
    User {
        id: 1,
        name: "amy".to_string(),
        email: "amy@example.com".to_string(),
    }
}

pub fn sample_address() -> Address {
    Address {
        street: "1 Main St".to_string(),
        city: "Springfield".to_string(),
        zip_code: Some("12345".to_string()),
    }
}

pub fn sample_user_profile() -> UserProfile {
    UserProfile {
        name: "amy".to_string(),
        age: 30,
        address: sample_address(),
    }
}

pub fn sample_product() -> Product {
    Product {
        id: 7,
        name: "widget".to_string(),
        price: 9.99,
    }
}

pub fn sample_employee() -> Employee {
    Employee {
        id: 3,
        profile: sample_user_profile(),
        skills: vec!["rust".to_string(), "go".to_string()],
        project_hours: HashMap::from([("core".to_string(), 40i64)]),
    }
}

pub fn sample_company() -> Company {
    Company {
        name: "acme".to_string(),
        employees: vec![Box::new(sample_user())],
    }
}

pub fn sample_nested_item() -> NestedItem {
    NestedItem {
        id: 1,
        map_data: HashMap::from([("k".to_string(), Value::from("v"))]),
        list_data: vec![Value::from(1i64), Value::from("x")],
    }
}

// ---------------------------------------------------------------------------
// object-package fixtures (mirror object/misc_test.go)
// ---------------------------------------------------------------------------

/// go's anonymous `struct { Name string; Value int }` items of ComplexData.
#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct ComplexDataItem {
    #[reflect(rename = "Name")]
    pub name: String,
    #[reflect(rename = "Value")]
    pub value: i64,
}

/// go's ComplexData: highly nested struct with maps, slices and a struct.
#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
pub struct ComplexData {
    #[reflect(rename = "ID")]
    pub id: i64,
    #[reflect(rename = "Details")]
    pub details: HashMap<String, Value>,
    #[reflect(rename = "Items")]
    pub items: Vec<ComplexDataItem>,
    #[reflect(rename = "User")]
    pub user: User,
}
