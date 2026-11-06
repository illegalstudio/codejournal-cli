use serde_json::Value;

pub fn render(data: &Value, lines: &mut Vec<String>) {
    lines.push(String::new());
    lines.push("Your forwarded tasks closed elsewhere:".to_owned());
    let info = &data["forwarded_results_info"];
    if info["status"] == "unavailable" {
        lines.push(format!(
            "  {}",
            info["reason"]
                .as_str()
                .unwrap_or("Unavailable for this brief.")
        ));
        return;
    }
    let rows = data["forwarded_results"].as_array();
    if rows.is_none_or(|rows| rows.is_empty()) {
        lines.push(format!("  {}", info["reason"].as_str().unwrap_or("(none)")));
    }
    for row in rows.into_iter().flatten().take(10) {
        let id = row["id"].as_str().unwrap_or("").replace('-', "");
        let destination = row["project_slug"]
            .as_str()
            .map(|slug| format!("{slug}: "))
            .unwrap_or_default();
        lines.push(format!(
            "  {}  {:<11} {}{}",
            &id[..id.len().min(8)],
            row["status"].as_str().unwrap_or(""),
            destination,
            row["title"].as_str().unwrap_or("")
        ));
    }
    if let Some(days) = info["window_days"].as_u64() {
        lines.push(format!(
            "  Latest 10 closures from the last {days} days. Older closures: cj brief --all."
        ));
    }
}
