use serde_json::Value;

pub fn item(row: &Value) -> String {
    format!(
        "{}  [{}] {}\n  {} ({}) | {}",
        row["id"].as_str().unwrap_or(""),
        row["project_slug"].as_str().unwrap_or("workspace"),
        row["title"].as_str().unwrap_or(""),
        row["due_at"].as_str().unwrap_or(""),
        row["timezone"].as_str().unwrap_or(""),
        row["status"].as_str().unwrap_or("open"),
    )
}

pub fn detail(data: &Value) -> String {
    let row = &data["reminder"];
    let mut lines = vec![item(row)];
    if let Some(body) = row["body"].as_str().filter(|body| !body.is_empty()) {
        lines.push(body.to_owned());
    }
    if let Some(url) = row["reference_url"].as_str() {
        lines.push(url.to_owned());
    }
    lines.join("\n\n")
}

pub fn brief(data: &Value, lines: &mut Vec<String>) {
    let Some(items) = data["reminders"]["items"]
        .as_array()
        .filter(|rows| !rows.is_empty())
    else {
        return;
    };
    lines.push("\nDue reminders (including other authorized projects):".into());
    for row in items {
        lines.push(format!("  {}", item(row).replace('\n', "\n  ")));
    }
    if data["reminders"]["total"].as_u64().unwrap_or(0) > items.len() as u64 {
        lines.push("  More due reminders: cj reminder list --state due".into());
    }
    lines.push("Claim with `cj reminder claim ID --json` before announcing. Only announce when claimed=true, then record it with `cj reminder presented ID --delivery-token TOKEN`. Reading or presenting never closes it. Follow the reminder protocol in the code-journal skill.".into());
}
