use serde_json::Value;

/// Later environment facts that share rare terms with the entry being read.
pub fn lines(entry: &Value) -> Vec<String> {
    let facts = entry["newer_environment"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    if facts.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![
        String::new(),
        "Newer environment facts sharing rare terms (check them before relying on operational details above):".to_owned(),
    ];
    lines.extend(facts.iter().map(|fact| {
        format!(
            "  {}  {}  {}",
            text(&fact["id"])
                .replace('-', "")
                .chars()
                .take(8)
                .collect::<String>(),
            text(&fact["created_at"])
                .chars()
                .take(10)
                .collect::<String>(),
            text(&fact["title"])
        )
    }));
    lines
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
