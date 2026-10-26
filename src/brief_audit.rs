use crate::brief_context_format;
use serde_json::{Value, json};

/// Keep the audit contract small even when talking to an older server.
pub fn project(data: &Value) -> Value {
    let mut result = fields(
        data,
        &[
            "rules",
            "counts",
            "mode",
            "pending_outbox",
            "locked",
            "notices",
        ],
    );
    result["audit"] = json!(true);
    result["project"] = fields(&data["project"], &["id", "slug", "name", "remote_url"]);
    for name in ["tasks", "plans", "docs", "other_sessions"] {
        let columns: &[&str] = if name == "other_sessions" {
            &[
                "id",
                "agent",
                "host",
                "checkout_path",
                "checkout_kind",
                "branch",
                "started_at",
                "last_seen_at",
                "waiting_since",
                "waiting_reason",
                "files",
                "prompts",
                "turns",
                "compactions",
            ]
        } else {
            &[
                "id",
                "title",
                "kind",
                "status",
                "priority",
                "revision",
                "not_before",
                "updated_at",
            ]
        };
        result[name] = json!(
            data[name]
                .as_array()
                .into_iter()
                .flatten()
                .map(|row| fields(row, columns))
                .collect::<Vec<_>>()
        );
    }
    result
}

fn fields(data: &Value, names: &[&str]) -> Value {
    let mut result = json!({});
    for name in names {
        if let Some(value) = data.get(name) {
            result[*name] = value.clone();
        }
    }
    result
}

pub fn render(data: &Value) -> String {
    let mut lines = vec![format!(
        "Repository audit: {}",
        data["project"]["slug"].as_str().unwrap_or("")
    )];
    if data["mode"] == "offline" {
        lines.push("Cached audit: session and work metadata may be outdated.".into());
    }
    for notice in data["notices"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        lines.push(notice.to_owned());
    }
    if data["locked"] == true {
        return lines.join("\n");
    }
    lines.push("\nProject rules (complete):".into());
    lines.push(
        data["rules"]
            .as_str()
            .filter(|rules| !rules.is_empty())
            .unwrap_or("(none set)")
            .to_owned(),
    );
    brief_context_format::active(data, &mut lines);
    for (name, title) in [
        ("tasks", "Open tasks"),
        ("plans", "Open plans"),
        ("docs", "Project docs"),
    ] {
        lines.push(format!("\n{title}:"));
        let rows = data[name].as_array().map(Vec::as_slice).unwrap_or_default();
        if rows.is_empty() {
            lines.push("  (none)".into());
        }
        for row in rows {
            lines.push(format!(
                "  {}  {}  {}",
                row["id"].as_str().unwrap_or(""),
                row["status"].as_str().unwrap_or(""),
                row["title"].as_str().unwrap_or("")
            ));
        }
    }
    lines.push("\nOpen relevant items with cj doc show, cj plan show or cj task show; use cj search QUERY for project knowledge.".into());
    lines.join("\n")
}
