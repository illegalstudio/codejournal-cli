use serde_json::Value;

/// Shared docs cover candidates only when their refs cover this project's knowledge.
pub fn covers(doc: &Value, entries: &[&Value]) -> bool {
    doc["scope"] != "global"
        || paths(doc).any(|reference| {
            entries.iter().flat_map(|entry| paths(entry)).any(|path| {
                let reference = reference.trim_matches('/');
                let path = path.trim_matches('/');
                !reference.is_empty()
                    && !path.is_empty()
                    && (reference == path
                        || path.starts_with(&format!("{reference}/"))
                        || reference.starts_with(&format!("{path}/")))
            })
        })
}

fn paths(item: &Value) -> impl Iterator<Item = &str> {
    item["refs"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|reference| reference["kind"] == "path")
        .filter_map(|reference| reference["value"].as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn shared_docs_require_matching_project_paths() {
        let doc = json!({"scope": "global", "refs": [{"kind": "path", "value": "src/recovery"}]});
        let relevant = json!({"refs": [{"kind": "path", "value": "src/recovery/runner.rs"}]});
        let unrelated = json!({"refs": [{"kind": "path", "value": "other/file.rs"}]});
        assert!(covers(&doc, &[&relevant]));
        assert!(!covers(&doc, &[&unrelated]));
        assert!(!covers(&doc, &[]));
        assert!(covers(&json!({"scope": "project"}), &[]));
    }
}
