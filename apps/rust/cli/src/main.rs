use dyn_rt::json::Value;

fn main() {
    let doc = Value::object([
        ("name", Value::from("dyn-rt")),
        ("version", Value::from(1u64)),
        ("contexts", Value::from(vec![Value::from("json")])),
    ]);
    println!("dyn-rt-cli: root is a {}, name = {:?}", doc.type_name(), doc.get("name"));
}
