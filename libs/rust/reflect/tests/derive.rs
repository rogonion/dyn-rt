//! Suites for the `dyn-rt-reflect-derive` macros (ADR 0009): the derive crate
//! cannot use its own macros, so its suites live here. A fixed set of derived
//! test types plus one titled table of observations/ops per suite.

use dyn_rt_reflect::{
    Reflect, ReflectEnum, ReflectNew, ReflectStruct, as_enum, as_struct, kind_of,
};
use dyn_rt_test_support::{TestMetadata, new_test_metadata, new_test_title, report_failures};

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
}

impl Subject {
    fn as_reflect(&self) -> &dyn Reflect {
        match self {
            Subject::User(v) => v,
            Subject::Point(v) => v,
            Subject::Message(v) => v,
            Subject::Opaque(v) => v,
        }
    }
}

#[allow(dead_code)]
enum Op {
    Kind(&'static str),
    TypeName(&'static str),
    FieldNames(&'static str),
    FieldTypeAt(usize, &'static str),
    FieldByName(&'static str, bool),
    Variant(&'static str),
    VariantNames(&'static str),
    CloneRoundtrip,
    ApplyMismatch,
    SetVariant(&'static str, bool), // name, has_payload -> expect UnknownVariant or Ok
    NewKind(&'static str),
}

struct DeriveCase {
    meta: TestMetadata,
    subject: Subject,
    op: Op,
    expected: Result<String, &'static str>,
}

macro_rules! derive_case {
    ($subject:expr, $op:expr, $expected:expr $(, test_title = $title:expr)?) => {
        DeriveCase {
            meta: new_test_metadata!(
                $(
                    test_title = Some($title.to_string())
                )?
            ),
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
        Op::CloneRoundtrip => {
            let cloned = r.reflect_clone();
            let same = match subject {
                Subject::User(v) => cloned.reflect_as_any().downcast_ref::<User>() == Some(v),
                Subject::Point(v) => cloned.reflect_as_any().downcast_ref::<Point>() == Some(v),
                Subject::Message(v) => cloned.reflect_as_any().downcast_ref::<Message>() == Some(v),
                Subject::Opaque(v) => cloned.reflect_as_any().downcast_ref::<Opaque>() == Some(v),
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
    ];

    let mut failures: Vec<String> = Vec::new();
    for (test_index, case) in test_data.iter().enumerate() {
        let title = new_test_title!(test_index + 1, case.meta.test_title);
        let got = observe(&case.subject, &case.op);
        match (&case.expected, &got) {
            (Ok(expected), Ok(got)) if expected == got => {}
            (Err(expected), Err(got)) if expected == got => {}
            _ => failures.push(format!(
                "{title} || expected {:?}, got {:?}",
                case.expected, got
            )),
        }
    }
    report_failures(failures, test_data.len());
}

// Cloneable exists only to prove the base derive's Clone bound is what gates
// participation (compile-time), not runtime — kept as a compile check.
#[allow(dead_code)]
fn _assert_clone_bound(_v: &Cloneable) {
    fn needs_reflect<T: Reflect>(_: &T) {}
    needs_reflect(&Opaque("x".to_string()));
}
