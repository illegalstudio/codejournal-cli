use serde_json::Value;

pub fn active(data: &Value, lines: &mut Vec<String>) {
    if let Some(items) = data["other_sessions"]
        .as_array()
        .filter(|items| !items.is_empty())
    {
        lines.push(String::new());
        lines.push("Other agents working here (sessions active in the last 2 hours; coordinate before touching the same files):".into());
        for row in items.iter().take(6) {
            let files = row["files"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .rev()
                .take(3)
                .collect::<Vec<_>>();
            let mut detail = String::new();
            if !row["waiting_since"].is_null() {
                detail.push_str("; waiting for the user");
            }
            if !files.is_empty() {
                detail.push_str(&format!(
                    "; editing {}",
                    files.into_iter().rev().collect::<Vec<_>>().join(", ")
                ));
            }
            lines.push(format!(
                "  {} on {} at {} ({}){detail}",
                value(&row["agent"]),
                row["branch"].as_str().unwrap_or("detached"),
                value(&row["checkout_path"]),
                value(&row["host"])
            ));
        }
    }
    if let Some(items) = data["delegations"]
        .as_array()
        .filter(|items| !items.is_empty())
    {
        lines.push(String::new());
        lines.push("Delegations in progress here:".into());
        for row in items {
            let id = value(&row["id"]).replace('-', "");
            lines.push(format!(
                "  {}  {}  {}  {}",
                &id[..id.len().min(8)],
                value(&row["status"]),
                value(&row["target"]),
                value(&row["task_title"])
            ));
        }
    }
}

pub fn global(data: &Value, lines: &mut Vec<String>) {
    if let Some(items) = data["global"].as_array().filter(|items| !items.is_empty()) {
        lines.push(String::new());
        lines.push("Global knowledge from other projects (shared topics):".into());
        for row in items {
            let id = value(&row["id"]).replace('-', "");
            lines.push(format!(
                "  {}  {}: {}",
                &id[..id.len().min(8)],
                value(&row["project_slug"]),
                value(&row["title"])
            ));
        }
    }
}

pub fn focus(data: &Value, lines: &mut Vec<String>, compact: bool) {
    let focus = &data["focus"];
    let entries = focus["entries"]
        .as_array()
        .filter(|items| !items.is_empty());
    let docs = focus["docs"].as_array().filter(|items| !items.is_empty());
    if entries.is_none() && docs.is_none() {
        return;
    }
    let count = focus["changes"]["files"].as_array().map_or(0, Vec::len);
    let where_text = match (
        focus["changes"]["branch"].as_str(),
        focus["changes"]["compared_to"].as_str(),
    ) {
        (Some(branch), Some(base)) => format!("branch {branch} vs {base}"),
        _ => "uncommitted changes".to_owned(),
    };
    lines.push(String::new());
    lines.push(format!(
        "Relevant to your changes ({where_text}, {count} files):"
    ));
    for doc in docs.into_iter().flatten() {
        let id = value(&doc["id"]).replace('-', "");
        lines.push(format!(
            "  {}  doc  {}",
            &id[..id.len().min(8)],
            value(&doc["title"])
        ));
    }
    for entry in entries
        .into_iter()
        .flatten()
        .take(if compact { 8 } else { 12 })
    {
        let id = value(&entry["id"]).replace('-', "");
        lines.push(format!(
            "  {}  {}  {}",
            &id[..id.len().min(8)],
            value(&entry["kind"]),
            value(&entry["title"])
        ));
    }
}

pub fn tail(data: &Value, lines: &mut Vec<String>, compact: bool) {
    if !compact {
        if let Some(items) = data["doc_candidates"]
            .as_array()
            .filter(|items| !items.is_empty())
        {
            let labels = items
                .iter()
                .map(|row| format!("#{} ({} entries)", value(&row["name"]), row["entries"]))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(String::new());
            lines.push(format!("Doc candidates (busy topics without a doc; write one with `cj doc create` when you work there): {labels}"));
        }
    }
    if let Some(hint) = data["garden_hint"].as_str() {
        lines.push(String::new());
        lines.push(hint.to_owned());
    }
    if compact {
        lines.push(String::new());
        lines.push("This is the compact brief. Run `cj brief` for docs, recent work, pinned knowledge, recent entries, topics, open questions, and global knowledge before investigating anything it may cover.".into());
    } else {
        lines.push(String::new());
        lines.push("Next: `cj show ID` or `cj plan show ID` for details, `cj search QUERY` before investigating, `cj log list --since yesterday` for recent work.".into());
    }
}

fn value(value: &Value) -> &str {
    value.as_str().unwrap_or("unknown")
}
