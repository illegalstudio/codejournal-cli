use serde_json::Value;

pub fn line(entry: &Value, all_projects: bool) -> String {
    let id = string(&entry["id"]).replace('-', "");
    let id = &id[..id.len().min(8)];
    let date = string(&entry["created_at"]);
    let date = &date[..date.len().min(10)];
    let topics = array_text(&entry["topics"]);
    let topic_suffix = if topics.is_empty() {
        String::new()
    } else {
        format!("  [{topics}]")
    };
    let status = string(&entry["status"]);
    let status_suffix = if status == "active" {
        String::new()
    } else {
        format!("  ({status})")
    };
    let global = if string(&entry["scope"]) == "global" {
        "  [global]"
    } else {
        ""
    };
    let wrong = entry["usage"]["wrong"].as_u64().unwrap_or(0);
    let wrong = if wrong > 0 {
        format!("  [reported wrong {wrong}x]")
    } else {
        String::new()
    };
    let prefix = if all_projects {
        format!("  [{}]", string(&entry["project_slug"]))
    } else {
        "  ".to_owned()
    };
    let mut line = format!(
        "{prefix}{id}  {:<12} {date}  {}{topic_suffix}{status_suffix}{global}{wrong}",
        string(&entry["kind"]),
        string(&entry["title"])
    );
    if let Some(excerpt) = entry["match_excerpt"]
        .as_str()
        .filter(|value| !value.is_empty())
    {
        line.push_str(&format!("\n      match: {excerpt}"));
    }
    line
}

fn string(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
fn array_text(value: &Value) -> String {
    value
        .as_array()
        .map(|items| items.iter().map(string).collect::<Vec<_>>().join(", "))
        .unwrap_or_default()
}
