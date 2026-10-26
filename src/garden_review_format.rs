use serde_json::Value;

pub fn sections(data: &Value, lines: &mut Vec<String>) {
    for (key, heading) in [
        (
            "possible_topics",
            "Topics that may be the same (merge with `cj topics merge A --into B` if so):",
        ),
        (
            "duplicates",
            "Entries that may duplicate each other (supersede the weaker with `cj supersede OLD --by NEW`):",
        ),
        (
            "stale_entries",
            "Entries whose referenced code moved on (verify, then supersede or mark obsolete; if a file was only renamed, `cj refs move OLD NEW`):",
        ),
        (
            "reported_wrong",
            "Entries reported wrong by an agent (verify, then supersede or mark obsolete):",
        ),
        (
            "doc_candidates",
            "Busy topics without a doc (distill the entries into one with `cj doc create`, citing them):",
        ),
        (
            "other_branch_entries",
            "For information: entries about files that live on another branch (not stale here):",
        ),
        (
            "stale_docs",
            "Docs whose code changed since their last update (reread and update):",
        ),
        (
            "idle_plans",
            "Active plans untouched for 14 days (update, mark done, or abandon):",
        ),
    ] {
        let Some(items) = data[key].as_array().filter(|items| !items.is_empty()) else {
            continue;
        };
        lines.push(format!("  {heading}"));
        let limit = match key {
            "possible_topics" => 20,
            "duplicates" | "other_branch_entries" => 15,
            "stale_entries" => 25,
            _ => usize::MAX,
        };
        for item in items.iter().take(limit) {
            lines.push(format_item(key, item, data));
        }
    }
}

fn format_item(key: &str, item: &Value, data: &Value) -> String {
    match key {
        "possible_topics" => {
            let a = value(&item[0]);
            let b = value(&item[1]);
            format!("    {a} ({}) ~ {b} ({})", count(data, a), count(data, b))
        }
        "duplicates" => format!(
            "    {} {}\n    {} {}  (similarity {})",
            short(&item["a"]["id"]),
            value(&item["a"]["title"]),
            short(&item["b"]["id"]),
            value(&item["b"]["title"]),
            item["score"]
        ),
        "stale_entries" | "other_branch_entries" => format!(
            "    {} {}{}",
            short(&item["id"]),
            value(&item["title"]),
            staleness_note(&item["staleness"])
        ),
        "reported_wrong" => format!(
            "    {} {}  [reported wrong {}x]",
            short(&item["id"]),
            value(&item["title"]),
            item["wrong"]
        ),
        "doc_candidates" => format!(
            "    #{} ({} entries)",
            value(&item["topic"]),
            item["entries"]
        ),
        "stale_docs" => format!(
            "    {} {} ({} commits)",
            short(&item["id"]),
            value(&item["title"]),
            item["code_changes"]
        ),
        "idle_plans" => format!(
            "    {} {} (updated {})",
            short(&item["id"]),
            value(&item["title"]),
            value(&item["updated_at"]).get(..10).unwrap_or("")
        ),
        _ => String::new(),
    }
}

fn staleness_note(stale: &Value) -> String {
    let mut reasons = Vec::new();
    if let Some(paths) = stale["gone"].as_array().filter(|paths| !paths.is_empty()) {
        reasons.push(format!(
            "path gone: {}",
            paths
                .iter()
                .take(2)
                .map(value)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if stale["changes"].as_u64().unwrap_or(0) >= 5 {
        reasons.push(format!("refs changed by {} commits", stale["changes"]));
    }
    let mut note = if reasons.is_empty() {
        String::new()
    } else {
        format!("  [verify: {}]", reasons.join("; "))
    };
    if let Some(other) = stale["elsewhere"]
        .as_array()
        .filter(|rows| !rows.is_empty())
    {
        let locations = other
            .iter()
            .take(2)
            .map(|row| format!("{} ({})", value(&row["path"]), value(&row["where"])))
            .collect::<Vec<_>>()
            .join(", ");
        note.push_str(&format!("  [on another branch: {locations}]"));
    }
    note
}

fn count(data: &Value, name: &str) -> u64 {
    data["topic_counts"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|row| row["name"] == name)
        .and_then(|row| row["count"].as_u64())
        .unwrap_or(0)
}

fn short(value: &Value) -> String {
    let id = value.as_str().unwrap_or("").replace('-', "");
    id.chars().take(8).collect()
}

fn value(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
