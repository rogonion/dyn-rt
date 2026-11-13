//! Suites for the `dyn-rt-reflect-derive` macros (ADR 0009): the derive crate
//! cannot use its own macros, so its suites live here. A fixed set of derived
//! test types plus one titled table of observations/ops per suite.

use dyn_rt_reflect::{
    Reflect, ReflectEnum, ReflectNew, ReflectStruct, as_enum, as_struct, kind_of,
};
use dyn_rt_test_support::{TestReport, new_test_title};

// ---------------------------------------------------------------------------
// derived test types
// ---------------------------------------------------------------------------

#[derive(ReflectStruct, Clone, Debug, PartialEq)]
#[reflect(name = "User")]
struct User {
    #[reflect(rename = "userName")]
    name: String,
    age: i64,
    #[reflect(skip)]
    session_token: String,
}

#[derive(ReflectStruct, Clone, Debug, PartialEq, Default)]
#[reflect(new)]
struct Point {
    x: f64,
    y: f64,
}

#[derive(ReflectEnum, Clone, Debug, PartialEq)]
enum Message {
    Text(String),
    #[reflect(rename = "empty")]
    Nothing,
}

#[derive(Reflect, Clone, Debug, PartialEq)]
struct Opaque(String);

// --- complex / non-terminal shapes: nested struct, embedded struct in an
// --- enum payload, recursive enum behind an indirection

#[derive(ReflectStruct, Clone, Debug, PartialEq)]
struct Address {
    street: String,
    city: String,
}

/// nested struct field: `address` is itself a ReflectStruct
#[derive(ReflectStruct, Clone, Debug, PartialEq)]
struct Person {
    name: String,
    address: Address,
}

/// enum with a struct payload (embedded struct as variant payload)
#[derive(ReflectEnum, Clone, Debug, PartialEq)]
enum Contact {
    Email(String),
    Home(Address),
}

/// recursive enum behind Box (indirection payload)
#[derive(ReflectEnum, Clone, Debug, PartialEq)]
enum Expr {
    Lit(i64),
    Wrap(Box<Expr>),
}

/// in-line struct within an enum, where the payload struct itself nests
/// another struct (enum -> struct -> struct); Rust has no anonymous
/// composite types, so the payload is a named struct used directly as the
/// variant payload
#[derive(ReflectEnum, Clone, Debug, PartialEq)]
enum Record {
    Person(Person),
    Empty,
}

/// struct variant: named inline fields on an enum variant (Rust's
/// equivalent of an anonymous struct inside an enum)
#[derive(ReflectEnum, Clone, Debug, PartialEq)]
enum Event {
    Quit,
    #[reflect(rename = "key")]
    KeyPress(char),
    Click {
        x: i64,
        y: i64,
    },
}

/// struct payload used to set a struct variant by field name
#[derive(ReflectStruct, Clone, Debug, PartialEq)]
struct Coords {
    x: i64,
    y: i64,
}

#[derive(Clone)]
#[allow(dead_code)]
struct Cloneable(String);

// ---------------------------------------------------------------------------
// case table: each op produces an observation string or an error kind
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum Subject {
    User(User),
    Point(Point),
    Message(Message),
    Opaque(Opaque),
    Person(Person),
    Contact(Contact),
    Expr(Expr),
    Record(Record),
    Event(Event),
}

impl Subject {
    fn as_reflect(&self) -> &dyn Reflect {
        match self {
            Subject::User(v) => v,
            Subject::Point(v) => v,
            Subject::Message(v) => v,
            Subject::Opaque(v) => v,
            Subject::Person(v) => v,
            Subject::Contact(v) => v,
            Subject::Expr(v) => v,
            Subject::Record(v) => v,
            Subject::Event(v) => v,
        }
    }
}

#[allow(dead_code)]
enum Op {
    Kind(&'static str),
    TypeName(&'static str),
    FieldNames(&'static str),
    FieldTypeAt(usize, &'static str),
    FieldKindAt(usize, &'static str),
    FieldByName(&'static str, bool),
    Variant(&'static str),
    VariantNames(&'static str),
    PayloadKind(&'static str),
    PayloadFieldNames(&'static str),
    PayloadFieldKindAt(usize, &'static str),
    VariantFieldNames(&'static str),
    VariantFieldKindAt(usize, &'static str),
    VariantFieldInfosByName(&'static str, &'static str),
    SetVariantFields(&'static str),
    CloneRoundtrip,
    ApplyMismatch,
    SetVariant(&'static str, bool), // name, has_payload -> expect UnknownVariant or Ok
    NewKind(&'static str),
}

struct DeriveCase {
    test_title: Option<String>,
    subject: Subject,
    op: Op,
    expected: Result<String, &'static str>,
}

macro_rules! derive_case {
    ($subject:expr, $op:expr, $expected:expr $(, test_title = $title:expr)?) => {
        DeriveCase {
test_title: None $(.or(Some($title.to_string())))?,
            subject: $subject,
            op: $op,
            expected: $expected,
        }
    };
}

fn observe(subject: &Subject, op: &Op) -> Result<String, &'static str> {
    let r = subject.as_reflect();
    match op {
        Op::Kind(expected) => {
            let _ = expected;
            Ok(kind_of(r).to_string())
        }
        Op::TypeName(_) => Ok(r.reflect_type_name().to_string()),
        Op::FieldNames(_) => {
            let s = as_struct(r).expect("struct");
            Ok(s.reflect_field_infos()
                .iter()
                .map(|f| f.name)
                .collect::<Vec<_>>()
                .join(","))
        }
        Op::FieldTypeAt(index, _) => {
            let s = as_struct(r).expect("struct");
            match s.reflect_field_at(*index) {
                Some(field) => Ok(field.reflect_type_name().to_string()),
                None => Ok("<none>".to_string()),
            }
        }
        Op::FieldKindAt(index, _) => {
            let s = as_struct(r).expect("struct");
            match s.reflect_field_at(*index) {
                Some(field) => Ok(kind_of(field).to_string()),
                None => Ok("<none>".to_string()),
            }
        }
        Op::FieldByName(name, _) => {
            let s = as_struct(r).expect("struct");
            Ok(if s.reflect_field(name).is_some() {
                "some"
            } else {
                "none"
            }
            .to_string())
        }
        Op::Variant(_) => {
            let e = as_enum(r).expect("enum");
            Ok(e.reflect_active_variant().to_string())
        }
        Op::VariantNames(_) => {
            let e = as_enum(r).expect("enum");
            Ok(e.reflect_variants()
                .iter()
                .map(|v| v.name)
                .collect::<Vec<_>>()
                .join(","))
        }
        Op::PayloadKind(_) => {
            let e = as_enum(r).expect("enum");
            match e.reflect_active_variant_payload() {
                Some(payload) => Ok(kind_of(payload).to_string()),
                None => Ok("<none>".to_string()),
            }
        }
        Op::PayloadFieldNames(_) => {
            let e = as_enum(r).expect("enum");
            let payload = e.reflect_active_variant_payload().expect("payload");
            let s = as_struct(payload).expect("struct payload");
            Ok(s.reflect_field_infos()
                .iter()
                .map(|f| f.name)
                .collect::<Vec<_>>()
                .join(","))
        }
        Op::PayloadFieldKindAt(index, _) => {
            let e = as_enum(r).expect("enum");
            let payload = e.reflect_active_variant_payload().expect("payload");
            let s = as_struct(payload).expect("struct payload");
            match s.reflect_field_at(*index) {
                Some(field) => Ok(kind_of(field).to_string()),
                None => Ok("<none>".to_string()),
            }
        }
        Op::VariantFieldNames(_) => {
            let e = as_enum(r).expect("enum");
            Ok(e.reflect_active_variant_field_infos()
                .iter()
                .map(|f| f.name)
                .collect::<Vec<_>>()
                .join(","))
        }
        Op::VariantFieldKindAt(index, _) => {
            let e = as_enum(r).expect("enum");
            match e.reflect_active_variant_field_at(*index) {
                Some(field) => Ok(kind_of(field).to_string()),
                None => Ok("<none>".to_string()),
            }
        }
        Op::VariantFieldInfosByName(variant_name, _) => {
            let e = as_enum(r).expect("enum");
            Ok(e.reflect_variant_field_infos(variant_name)
                .iter()
                .map(|f| f.name)
                .collect::<Vec<_>>()
                .join(","))
        }
        Op::SetVariantFields(variant_name) => {
            let mut target = subject.clone();
            let mut_ref: &mut dyn Reflect = match &mut target {
                Subject::Event(v) => v,
                _ => panic!("SetVariantFields op only on Event"),
            };
            let result = match mut_ref.reflect_ref_mut() {
                dyn_rt_reflect::ReflectRefMut::Enum(e) => {
                    e.reflect_set_variant(variant_name, Some(Box::new(Coords { x: 9, y: 9 })))
                }
                _ => panic!("enum expected"),
            };
            match result {
                Ok(()) => {
                    let landed = matches!(&target, Subject::Event(Event::Click { x: 9, y: 9 }));
                    Ok(if landed { "equal" } else { "different" }.to_string())
                }
                Err(err) => Err(match err {
                    dyn_rt_reflect::ReflectError::ShapeMismatch { .. } => "ShapeMismatch",
                    dyn_rt_reflect::ReflectError::UnknownVariant { .. } => "UnknownVariant",
                    other_err => panic!("unexpected error: {other_err}"),
                }),
            }
        }
        Op::CloneRoundtrip => {
            let cloned = r.reflect_clone();
            let same = match subject {
                Subject::User(v) => cloned.reflect_as_any().downcast_ref::<User>() == Some(v),
                Subject::Point(v) => cloned.reflect_as_any().downcast_ref::<Point>() == Some(v),
                Subject::Message(v) => cloned.reflect_as_any().downcast_ref::<Message>() == Some(v),
                Subject::Opaque(v) => cloned.reflect_as_any().downcast_ref::<Opaque>() == Some(v),
                Subject::Person(v) => cloned.reflect_as_any().downcast_ref::<Person>() == Some(v),
                Subject::Contact(v) => cloned.reflect_as_any().downcast_ref::<Contact>() == Some(v),
                Subject::Expr(v) => cloned.reflect_as_any().downcast_ref::<Expr>() == Some(v),
                Subject::Record(v) => cloned.reflect_as_any().downcast_ref::<Record>() == Some(v),
                Subject::Event(v) => cloned.reflect_as_any().downcast_ref::<Event>() == Some(v),
            };
            Ok(if same { "equal" } else { "different" }.to_string())
        }
        Op::ApplyMismatch => {
            // apply a source of a different concrete type: Opaque for the
            // struct/enum subjects, String for the Opaque subject
            let opaque_source = Opaque("x".to_string());
            let string_source = "x".to_string();
            let mut target = subject.clone();
            let mut_ref: &mut dyn Reflect = match &mut target {
                Subject::User(v) => v,
                Subject::Point(v) => v,
                Subject::Message(v) => v,
                Subject::Opaque(v) => v,
                Subject::Person(v) => v,
                Subject::Contact(v) => v,
                Subject::Expr(v) => v,
                Subject::Record(v) => v,
                Subject::Event(v) => v,
            };
            let result = match subject {
                Subject::Opaque(_) => mut_ref.reflect_apply(&string_source),
                _ => mut_ref.reflect_apply(&opaque_source),
            };
            match result {
                Ok(()) => Ok("ok".to_string()),
                Err(err) => Err(match err {
                    dyn_rt_reflect::ReflectError::ShapeMismatch { .. } => "ShapeMismatch",
                    other_err => panic!("unexpected error: {other_err}"),
                }),
            }
        }
        Op::SetVariant(name, has_payload) => {
            let mut target = subject.clone();
            let mut_ref: &mut dyn Reflect = match &mut target {
                Subject::Message(v) => v,
                _ => panic!("SetVariant op only on Message"),
            };
            let result = match mut_ref.reflect_ref_mut() {
                dyn_rt_reflect::ReflectRefMut::Enum(e) => {
                    if *has_payload {
                        e.reflect_set_variant(name, Some(Box::new("hi".to_string())))
                    } else {
                        e.reflect_set_variant(name, None)
                    }
                }
                _ => panic!("expected enum"),
            };
            match result {
                Ok(()) => Ok(as_enum(mut_ref)
                    .expect("enum")
                    .reflect_active_variant()
                    .to_string()),
                Err(err) => Err(match err {
                    dyn_rt_reflect::ReflectError::UnknownVariant { .. } => "UnknownVariant",
                    dyn_rt_reflect::ReflectError::ShapeMismatch { .. } => "ShapeMismatch",
                    other_err => panic!("unexpected error: {other_err}"),
                }),
            }
        }
        Op::NewKind(_) => {
            let new: Box<dyn Reflect> = match subject {
                Subject::Point(_) => Point::reflect_new(),
                _ => panic!("NewKind op only on Point (#[reflect(new)])"),
            };
            Ok(kind_of(new.as_ref()).to_string())
        }
    }
}

#[test]
fn derive_ops() {
    let test_data: Vec<_> = vec![
        derive_case!(
            Subject::User(User {
                name: "amy".to_string(),
                age: 30,
                session_token: "t".to_string()
            }),
            Op::Kind("struct"),
            Ok("struct".to_string()),
            test_title = "derived struct routes to Struct kind"
        ),
        derive_case!(
            Subject::User(User {
                name: "amy".to_string(),
                age: 30,
                session_token: "t".to_string()
            }),
            Op::TypeName("User"),
            Ok("User".to_string()),
            test_title = "#[reflect(name)] overrides type name"
        ),
        derive_case!(
            Subject::User(User {
                name: "amy".to_string(),
                age: 30,
                session_token: "t".to_string()
            }),
            Op::FieldNames(""),
            Ok("userName,age".to_string()),
            test_title = "field table uses rename and excludes skip"
        ),
        derive_case!(
            Subject::User(User {
                name: "amy".to_string(),
                age: 30,
                session_token: "t".to_string()
            }),
            Op::FieldByName("userName", true),
            Ok("some".to_string()),
            test_title = "renamed field found by external name"
        ),
        derive_case!(
            Subject::User(User {
                name: "amy".to_string(),
                age: 30,
                session_token: "t".to_string()
            }),
            Op::FieldByName("name", false),
            Ok("none".to_string()),
            test_title = "original field name is hidden after rename"
        ),
        derive_case!(
            Subject::User(User {
                name: "amy".to_string(),
                age: 30,
                session_token: "t".to_string()
            }),
            Op::FieldByName("session_token", false),
            Ok("none".to_string()),
            test_title = "skipped field is not in the table"
        ),
        derive_case!(
            Subject::User(User {
                name: "amy".to_string(),
                age: 30,
                session_token: "t".to_string()
            }),
            Op::FieldTypeAt(1, "i64"),
            Ok("i64".to_string()),
            test_title = "field at index 1 is the i64 age"
        ),
        derive_case!(
            Subject::User(User {
                name: "amy".to_string(),
                age: 30,
                session_token: "t".to_string()
            }),
            Op::CloneRoundtrip,
            Ok("equal".to_string()),
            test_title =
                "structural clone of derived struct equals original (skip field via Clone)"
        ),
        derive_case!(
            Subject::User(User {
                name: "amy".to_string(),
                age: 30,
                session_token: "t".to_string()
            }),
            Op::ApplyMismatch,
            Err("ShapeMismatch"),
            test_title = "apply from a different concrete type fails ShapeMismatch"
        ),
        derive_case!(
            Subject::Point(Point { x: 1.0, y: 2.0 }),
            Op::Kind("struct"),
            Ok("struct".to_string()),
            test_title = "Point routes to Struct kind"
        ),
        derive_case!(
            Subject::Point(Point { x: 1.0, y: 2.0 }),
            Op::NewKind("struct"),
            Ok("struct".to_string()),
            test_title = "#[reflect(new)] emits ReflectNew"
        ),
        derive_case!(
            Subject::Message(Message::Text("hello".to_string())),
            Op::Variant("Text"),
            Ok("Text".to_string()),
            test_title = "enum active variant"
        ),
        derive_case!(
            Subject::Message(Message::Nothing),
            Op::VariantNames(""),
            Ok("Text,empty".to_string()),
            test_title = "variant table uses rename"
        ),
        derive_case!(
            Subject::Message(Message::Nothing),
            Op::SetVariant("Text", true),
            Ok("Text".to_string()),
            test_title = "set_variant switches to payload variant"
        ),
        derive_case!(
            Subject::Message(Message::Text("hi".to_string())),
            Op::SetVariant("empty", false),
            Ok("empty".to_string()),
            test_title = "set_variant switches to renamed unit variant"
        ),
        derive_case!(
            Subject::Message(Message::Text("hi".to_string())),
            Op::SetVariant("Nope", false),
            Err("UnknownVariant"),
            test_title = "set_variant unknown name errors UnknownVariant"
        ),
        derive_case!(
            Subject::Message(Message::Text("hi".to_string())),
            Op::CloneRoundtrip,
            Ok("equal".to_string()),
            test_title = "structural clone of derived enum equals original"
        ),
        derive_case!(
            Subject::Opaque(Opaque("secret".to_string())),
            Op::Kind("unknown"),
            Ok("unknown".to_string()),
            test_title = "base Reflect derive routes to Unknown"
        ),
        derive_case!(
            Subject::Opaque(Opaque("secret".to_string())),
            Op::CloneRoundtrip,
            Ok("equal".to_string()),
            test_title = "base derive clones via Clone"
        ),
        derive_case!(
            Subject::Opaque(Opaque("secret".to_string())),
            Op::ApplyMismatch,
            Err("ShapeMismatch"),
            test_title = "base derive apply from other type fails ShapeMismatch"
        ),
        // --- nested struct: Person has an Address struct field
        derive_case!(
            Subject::Person(Person {
                name: "amy".to_string(),
                address: Address {
                    street: "1 Main St".to_string(),
                    city: "Springfield".to_string()
                }
            }),
            Op::Kind("struct"),
            Ok("struct".to_string()),
            test_title = "nested struct subject routes to Struct kind"
        ),
        derive_case!(
            Subject::Person(Person {
                name: "amy".to_string(),
                address: Address {
                    street: "1 Main St".to_string(),
                    city: "Springfield".to_string()
                }
            }),
            Op::FieldNames(""),
            Ok("name,address".to_string()),
            test_title = "nested struct field table lists all fields"
        ),
        derive_case!(
            Subject::Person(Person {
                name: "amy".to_string(),
                address: Address {
                    street: "1 Main St".to_string(),
                    city: "Springfield".to_string()
                }
            }),
            Op::FieldKindAt(1, "struct"),
            Ok("struct".to_string()),
            test_title = "nested struct field at index 1 has Struct kind"
        ),
        derive_case!(
            Subject::Person(Person {
                name: "amy".to_string(),
                address: Address {
                    street: "1 Main St".to_string(),
                    city: "Springfield".to_string()
                }
            }),
            Op::CloneRoundtrip,
            Ok("equal".to_string()),
            test_title = "clone of struct with nested struct field equals original"
        ),
        // --- enum with embedded struct payload: Contact::Home(Address)
        derive_case!(
            Subject::Contact(Contact::Home(Address {
                street: "2 Oak Ave".to_string(),
                city: "Shelbyville".to_string()
            })),
            Op::Kind("enum"),
            Ok("enum".to_string()),
            test_title = "enum with struct payload routes to Enum kind"
        ),
        derive_case!(
            Subject::Contact(Contact::Home(Address {
                street: "2 Oak Ave".to_string(),
                city: "Shelbyville".to_string()
            })),
            Op::Variant("Home"),
            Ok("Home".to_string()),
            test_title = "active variant of enum with struct payload"
        ),
        derive_case!(
            Subject::Contact(Contact::Home(Address {
                street: "2 Oak Ave".to_string(),
                city: "Shelbyville".to_string()
            })),
            Op::PayloadKind("struct"),
            Ok("struct".to_string()),
            test_title = "payload of struct-carrying variant has Struct kind"
        ),
        derive_case!(
            Subject::Contact(Contact::Email("a@b.c".to_string())),
            Op::PayloadKind("scalar"),
            Ok("scalar".to_string()),
            test_title = "payload of scalar-carrying variant has Scalar kind"
        ),
        derive_case!(
            Subject::Contact(Contact::Home(Address {
                street: "2 Oak Ave".to_string(),
                city: "Shelbyville".to_string()
            })),
            Op::CloneRoundtrip,
            Ok("equal".to_string()),
            test_title = "clone of enum with struct payload equals original"
        ),
        // --- recursive enum behind Box indirection: Expr::Wrap(Box<Expr>)
        derive_case!(
            Subject::Expr(Expr::Wrap(Box::new(Expr::Lit(7)))),
            Op::Variant("Wrap"),
            Ok("Wrap".to_string()),
            test_title = "recursive enum active variant"
        ),
        derive_case!(
            Subject::Expr(Expr::Wrap(Box::new(Expr::Lit(7)))),
            Op::PayloadKind("indirection"),
            Ok("indirection".to_string()),
            test_title = "Box payload of recursive variant has Indirection kind"
        ),
        derive_case!(
            Subject::Expr(Expr::Wrap(Box::new(Expr::Lit(7)))),
            Op::CloneRoundtrip,
            Ok("equal".to_string()),
            test_title = "clone of recursive enum through Box equals original"
        ),
        // --- in-line struct within an enum: Contact::Home(Address)
        derive_case!(
            Subject::Contact(Contact::Home(Address {
                street: "2 Oak Ave".to_string(),
                city: "Shelbyville".to_string()
            })),
            Op::PayloadFieldNames(""),
            Ok("street,city".to_string()),
            test_title = "struct payload of enum variant exposes its field table"
        ),
        // --- enum -> struct -> struct: Record::Person(Person{.., address: Address})
        derive_case!(
            Subject::Record(Record::Person(Person {
                name: "amy".to_string(),
                address: Address {
                    street: "1 Main St".to_string(),
                    city: "Springfield".to_string()
                }
            })),
            Op::Kind("enum"),
            Ok("enum".to_string()),
            test_title = "enum with nested-struct payload routes to Enum kind"
        ),
        derive_case!(
            Subject::Record(Record::Person(Person {
                name: "amy".to_string(),
                address: Address {
                    street: "1 Main St".to_string(),
                    city: "Springfield".to_string()
                }
            })),
            Op::Variant("Person"),
            Ok("Person".to_string()),
            test_title = "active variant of enum carrying a struct payload"
        ),
        derive_case!(
            Subject::Record(Record::Person(Person {
                name: "amy".to_string(),
                address: Address {
                    street: "1 Main St".to_string(),
                    city: "Springfield".to_string()
                }
            })),
            Op::PayloadKind("struct"),
            Ok("struct".to_string()),
            test_title = "payload of Record::Person has Struct kind"
        ),
        derive_case!(
            Subject::Record(Record::Person(Person {
                name: "amy".to_string(),
                address: Address {
                    street: "1 Main St".to_string(),
                    city: "Springfield".to_string()
                }
            })),
            Op::PayloadFieldNames(""),
            Ok("name,address".to_string()),
            test_title = "payload field table of Record::Person lists name,address"
        ),
        derive_case!(
            Subject::Record(Record::Person(Person {
                name: "amy".to_string(),
                address: Address {
                    street: "1 Main St".to_string(),
                    city: "Springfield".to_string()
                }
            })),
            Op::PayloadFieldKindAt(1, "struct"),
            Ok("struct".to_string()),
            test_title = "field inside enum payload is itself a nested struct"
        ),
        derive_case!(
            Subject::Record(Record::Person(Person {
                name: "amy".to_string(),
                address: Address {
                    street: "1 Main St".to_string(),
                    city: "Springfield".to_string()
                }
            })),
            Op::CloneRoundtrip,
            Ok("equal".to_string()),
            test_title = "clone of enum with nested-struct payload equals original"
        ),
        derive_case!(
            Subject::Record(Record::Empty),
            Op::PayloadKind("<none>"),
            Ok("<none>".to_string()),
            test_title = "unit variant of enum has no payload"
        ),
        // --- struct variant: Event::Click { x, y }
        derive_case!(
            Subject::Event(Event::Quit),
            Op::Kind("enum"),
            Ok("enum".to_string()),
            test_title = "enum with struct variant routes to Enum kind"
        ),
        derive_case!(
            Subject::Event(Event::Quit),
            Op::VariantNames(""),
            Ok("Quit,key,Click".to_string()),
            test_title = "variant table includes struct variant and applies rename"
        ),
        derive_case!(
            Subject::Event(Event::Quit),
            Op::Variant("Quit"),
            Ok("Quit".to_string()),
            test_title = "unit variant active"
        ),
        derive_case!(
            Subject::Event(Event::Quit),
            Op::PayloadKind("<none>"),
            Ok("<none>".to_string()),
            test_title = "unit variant has no payload"
        ),
        derive_case!(
            Subject::Event(Event::Quit),
            Op::VariantFieldNames(""),
            Ok("".to_string()),
            test_title = "unit variant has no fields"
        ),
        derive_case!(
            Subject::Event(Event::KeyPress('a')),
            Op::Variant("key"),
            Ok("key".to_string()),
            test_title = "renamed tuple variant active"
        ),
        derive_case!(
            Subject::Event(Event::KeyPress('a')),
            Op::PayloadKind("scalar"),
            Ok("scalar".to_string()),
            test_title = "char payload of tuple variant has Scalar kind"
        ),
        derive_case!(
            Subject::Event(Event::Click { x: 1, y: 2 }),
            Op::Variant("Click"),
            Ok("Click".to_string()),
            test_title = "struct variant active"
        ),
        derive_case!(
            Subject::Event(Event::Click { x: 1, y: 2 }),
            Op::PayloadKind("<none>"),
            Ok("<none>".to_string()),
            test_title = "struct variant has no single payload (fields instead)"
        ),
        derive_case!(
            Subject::Event(Event::Click { x: 1, y: 2 }),
            Op::VariantFieldNames(""),
            Ok("x,y".to_string()),
            test_title = "struct variant exposes its inline field table"
        ),
        derive_case!(
            Subject::Event(Event::Click { x: 1, y: 2 }),
            Op::VariantFieldKindAt(1, "scalar"),
            Ok("scalar".to_string()),
            test_title = "inline field of struct variant has Scalar kind"
        ),
        derive_case!(
            Subject::Event(Event::Click { x: 1, y: 2 }),
            Op::VariantFieldInfosByName("Click", ""),
            Ok("x,y".to_string()),
            test_title = "field table of struct variant by variant name"
        ),
        derive_case!(
            Subject::Event(Event::Click { x: 1, y: 2 }),
            Op::VariantFieldInfosByName("Quit", ""),
            Ok("".to_string()),
            test_title = "field table of unit variant by name is empty"
        ),
        derive_case!(
            Subject::Event(Event::Click { x: 1, y: 2 }),
            Op::CloneRoundtrip,
            Ok("equal".to_string()),
            test_title = "structural clone of struct variant equals original"
        ),
        derive_case!(
            Subject::Event(Event::Click { x: 1, y: 2 }),
            Op::ApplyMismatch,
            Err("ShapeMismatch"),
            test_title = "apply from a different concrete type fails ShapeMismatch"
        ),
        derive_case!(
            Subject::Event(Event::Quit),
            Op::SetVariantFields("Click"),
            Ok("equal".to_string()),
            test_title = "set_variant on struct variant builds fields from a struct payload"
        ),
        derive_case!(
            Subject::Event(Event::Quit),
            Op::SetVariantFields("Nope"),
            Err("UnknownVariant"),
            test_title = "set_variant with unknown name errors UnknownVariant"
        ),
    ];

    let mut report = TestReport::new(
        "derive_ops",
        "Suites for the `dyn-rt-reflect-derive` macros (ADR 0009): the derive crate",
    );
    for case in &test_data {
        let title = new_test_title!(case.test_title);
        report.begin_case(title.clone());
        let got = observe(&case.subject, &case.op);
        match (&case.expected, &got) {
            (Ok(expected), Ok(got)) if expected == got => {}
            (Err(expected), Err(got)) if expected == got => {}
            _ => report.fail_case(format!("expected {:?}, got {:?}", case.expected, got)),
        }
    }
    report.finish();
}

// Cloneable exists only to prove the base derive's Clone bound is what gates
// participation (compile-time), not runtime — kept as a compile check.
#[allow(dead_code)]
fn _assert_clone_bound(_v: &Cloneable) {
    fn needs_reflect<T: Reflect>(_: &T) {}
    needs_reflect(&Opaque("x".to_string()));
}
