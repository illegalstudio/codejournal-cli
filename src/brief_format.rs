use serde_json::Value;

pub fn render(
    data: &Value,
    compact: bool,
    limit: u32,
    pinned_limit: u32,
    log_limit: u32,
) -> String {
    let project = &data["project"];
    let mut lines = vec![format!(
        "Project: {} ({})",
        value(&project["slug"]),
        project["remote_url"].as_str().unwrap_or("no remote")
    )];
    let summary = data["counts"]
        .as_object()
        .map(|counts| {
            counts
                .iter()
                .map(|(name, count)| format!("{count} {name}"))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    lines.push(format!(
        "Entries: {} | Storage: remote | Pending outbox: 0",
        if summary.is_empty() {
            "0 entries"
        } else {
            &summary
        }
    ));
    lines.push(String::new());
    lines.push("Project rules (follow for the whole session):".into());
    if let Some(rules) = data["rules"].as_str().filter(|text| !text.is_empty()) {
        lines.extend(rules.lines().map(|line| format!("  {line}")));
    } else {
        lines.push("  (none set: write them now with `cj rules set`)".into());
    }
    section(&mut lines, "Now due:", &data["now_due"], 20);
    section(
        &mut lines,
        "Open tasks:",
        &data["tasks"],
        if compact { 5 } else { 10 },
    );
    section(
        &mut lines,
        "Your forwarded tasks closed elsewhere:",
        &data["forwarded_results"],
        10,
    );
    section(&mut lines, "Open plans:", &data["plans"], 20);
    if !compact {
        section(
            &mut lines,
            "Docs (open with `cj doc show ID`):",
            &data["docs"],
            20,
        );
        section(
            &mut lines,
            &format!("Recent work (last {log_limit} logs):"),
            &data["logs"],
            log_limit as usize,
        );
        section(&mut lines, "Open questions:", &data["questions"], 20);
        section(
            &mut lines,
            "Architecture, environment, and decisions (active):",
            &data["pinned"],
            pinned_limit as usize,
        );
        section(
            &mut lines,
            &format!("Recent entries (up to {limit}):"),
            &data["recent"],
            limit as usize,
        );
        let topics = data["topics"]
            .as_array()
            .map(|rows| {
                rows.iter()
                    .map(|row| format!("{} ({})", value(&row["name"]), row["count"]))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        lines.push(format!(
            "Topics: {}",
            if topics.is_empty() { "(none)" } else { &topics }
        ));
    }
    lines.join("\n")
}

fn section(lines: &mut Vec<String>, title: &str, items: &Value, limit: usize) {
    lines.push(String::new());
    lines.push(title.to_owned());
    let Some(items) = items.as_array() else {
        lines.push("  (none)".into());
        return;
    };
    if items.is_empty() {
        lines.push("  (none)".into());
    }
    for row in items.iter().take(limit) {
        let id = value(&row["id"]).replace('-', "");
        lines.push(format!(
            "  {}  {:<11} {}",
            &id[..id.len().min(8)],
            value(&row["status"]),
            value(&row["title"])
        ));
    }
    if items.len() > limit {
        lines.push(format!("  ... and {} more", items.len() - limit));
    }
}

fn value(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
