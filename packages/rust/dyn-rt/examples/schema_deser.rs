//! 2-step deserialization demo: parse JSON/YAML/TOML text into a
//! reflect-able serde value (adapters, `deser` feature), then convert it
//! into a typed struct via the schema (application/conversion.rs).
//!
//! Run: cargo run -p dyn-rt --example schema_deser --features deser

use dyn_rt::json::schema::{
    Schema, SchemaNode, convert, from_json_str, from_toml_str, from_yaml_str,
};
use dyn_rt::reflect::{Reflect, ReflectStruct};

#[derive(ReflectStruct, Clone, Debug, Default, PartialEq)]
#[reflect(new)]
struct User {
    #[reflect(rename = "ID")]
    id: i64,
    #[reflect(rename = "Name")]
    name: String,
    #[reflect(rename = "Email")]
    email: String,
}

fn user_schema() -> Schema {
    Schema::Node(SchemaNode::struct_node::<User>(vec![
        ("ID", Schema::Node(SchemaNode::scalar::<i64>())),
        ("Name", Schema::Node(SchemaNode::scalar::<String>())),
        ("Email", Schema::Node(SchemaNode::scalar::<String>())),
    ]))
}

fn main() {
    let texts = [
        (
            "json",
            r#"{"ID": 1, "Name": "amy", "Email": "amy@example.com"}"#,
        ),
        ("yaml", "ID: 1\nName: amy\nEmail: amy@example.com\n"),
        (
            "toml",
            "ID = 1\nName = \"amy\"\nEmail = \"amy@example.com\"\n",
        ),
    ];

    for (format, text) in texts {
        // step 1: text -> reflect-able serde value (serde_json::Value under
        // the hood, via the serde-json adapter in dyn-rt-reflect)
        let parsed: Box<dyn Reflect> = match format {
            "json" => from_json_str(text),
            "yaml" => from_yaml_str(text),
            "toml" => from_toml_str(text),
            _ => unreachable!(),
        }
        .expect("parse");

        // step 2: schema-driven conversion into the typed target
        let mut user = User::default();
        convert(&*parsed, &user_schema(), &mut user).expect("convert");

        println!(
            "{format}: parsed as '{}' then converted -> {user:?}",
            parsed.reflect_type_name()
        );
        assert_eq!(
            user,
            User {
                id: 1,
                name: "amy".to_string(),
                email: "amy@example.com".to_string()
            }
        );
    }
    println!("all formats deserialized and converted into User");
}
