use dyn_rt::json::{self, Value};

fn main() {
    let doc = Value::object([
        ("name", Value::from("dyn-rt")),
        ("version", Value::from(1u64)),
        ("contexts", Value::from(vec![Value::from("json")])),
    ]);
    println!(
        "dyn-rt-cli: root is a {}, name = {:?}",
        doc.type_name(),
        doc.get("name")
    );

    let args: Vec<String> = std::env::args().skip(1).collect();
    let paths = if args.is_empty() {
        vec![
            "$.store.book[0].title".to_string(),
            "$..['a..b']..id".to_string(),
        ]
    } else {
        args
    };

    for path in paths {
        let segments = json::parse(&path);
        println!(
            "parse {path} -> {} descent group(s), roundtrip = {}",
            segments.0.len(),
            segments
        );
    }
}
