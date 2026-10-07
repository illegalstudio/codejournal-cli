use serde_json::Value;

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}

pub fn append(lines: &mut Vec<String>, result: &Value, noun: &str, id: &str, content: bool) {
    lines.extend(["".to_owned(), "History:".to_owned()]);
    for record in result["revisions"].as_array().into_iter().flatten() {
        lines.push(format!(
            "  rev {} {} {}",
            record["revision"],
            text(&record["created_at"]),
            text(&record["note"])
        ));
        if content {
            lines.extend([
                text(&record["title"]).to_owned(),
                text(&record["body"]).to_owned(),
                "".to_owned(),
            ]);
        }
    }
    if let Some(before) = result["revision_next"].as_u64() {
        lines.push(format!(
            "More revisions: cj {noun} show {id} --history --before-revision {before}"
        ));
    }
}
