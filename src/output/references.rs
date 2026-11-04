use serde_json::Value;

/// Older server work-item payloads encoded reference lists as JSON strings.
pub fn normalize(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(encoded)) = map.get("refs") {
                if let Ok(Value::Array(items)) = serde_json::from_str::<Value>(encoded) {
                    if items
                        .iter()
                        .all(|item| item["kind"].is_string() && item["value"].is_string())
                    {
                        map.insert("refs".to_owned(), Value::Array(items));
                    }
                }
            }
            for item in map.values_mut() {
                normalize(item);
            }
        }
        Value::Array(items) => {
            for item in items {
                normalize(item);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn work_item_refs_are_arrays_without_changing_embedded_markdown() {
        let encoded = r#"[{"kind":"path","value":"src/app.rs"}]"#;
        let mut value = json!({"doc":{"refs":encoded,"body":encoded},"history":[{"refs":"[]"}]});
        normalize(&mut value);
        assert_eq!(
            value["doc"]["refs"],
            json!([{"kind":"path","value":"src/app.rs"}])
        );
        assert_eq!(value["doc"]["body"], encoded);
        assert_eq!(value["history"][0]["refs"], json!([]));
    }
}
