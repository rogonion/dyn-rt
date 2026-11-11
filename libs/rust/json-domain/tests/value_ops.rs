//! Data-driven suite for the Value model. Vectors: tests/shared/json/value-ops.json (ADR 0004).

use dyn_rt_test_support as support;

#[test]
fn value_ops() {
    for case in support::load_cases("json/value-ops.json") {
        let name = case["name"].as_str().unwrap_or("<unnamed>");
        let value = support::build_value(&case["build"]);
        let assert = &case["assert"];
        let ctx = || {
            format!(
                "case '{name}' failed:\n{}",
                serde_json::to_string_pretty(&case).unwrap()
            )
        };

        if let Some(t) = assert.get("type") {
            assert_eq!(value.type_name(), t.as_str().unwrap(), "{}", ctx());
        }
        if let Some(b) = assert.get("is_null") {
            assert_eq!(value.is_null(), b.as_bool().unwrap(), "{}", ctx());
        }
        if let Some(b) = assert.get("as_bool") {
            assert_eq!(value.as_bool(), Some(b.as_bool().unwrap()), "{}", ctx());
        }
        if let Some(n) = assert.get("as_f64") {
            assert_eq!(value.as_f64(), Some(n.as_f64().unwrap()), "{}", ctx());
        }
        if let Some(s) = assert.get("as_str") {
            assert_eq!(value.as_str(), Some(s.as_str().unwrap()), "{}", ctx());
        }
        if let Some(b) = assert.get("is_integer") {
            assert_eq!(
                value.as_number().map(|n| n.is_integer()),
                Some(b.as_bool().unwrap()),
                "{}",
                ctx()
            );
        }
        if let Some(len) = assert.get("as_array_len") {
            match len.as_u64() {
                Some(l) => assert_eq!(
                    value.as_array().map(<[dyn_rt_json_domain::Value]>::len),
                    Some(l as usize),
                    "{}",
                    ctx()
                ),
                None => assert!(value.as_array().is_none(), "{}", ctx()),
            }
        }
        if let Some(get) = assert.get("get").and_then(|g| g.as_object()) {
            for (k, expected) in get {
                assert_eq!(
                    value.get(k),
                    Some(&support::to_value(expected)),
                    "get '{k}' {}",
                    ctx()
                );
            }
        }
        if let Some(missing) = assert.get("get_missing").and_then(|m| m.as_array()) {
            for k in missing {
                let k = k.as_str().unwrap();
                assert!(value.get(k).is_none(), "get_missing '{k}' {}", ctx());
            }
        }
        if let Some(expected_obj) = assert.get("expected_object") {
            // string equality on compact JSON also verifies object key order
            assert_eq!(
                support::compact_value(&value),
                support::compact(expected_obj),
                "{}",
                ctx()
            );
        }
    }
}
