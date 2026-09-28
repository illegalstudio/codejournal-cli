use serde_json::Value;

pub fn render(data: &Value) -> String {
    let mode = if data["dry_run"] == true {
        "dry run, nothing written"
    } else {
        "safe fixes applied"
    };
    let mut lines = vec![
        format!(
            "Garden report for {} ({mode})",
            data["project"].as_str().unwrap_or("")
        ),
        String::new(),
    ];
    lines.push(
        if data["dry_run"] == true {
            "Would apply automatically:"
        } else {
            "Applied automatically:"
        }
        .into(),
    );
    if data["dry_run"] == true {
        let secrets = data["secrets"].as_array().map_or(0, Vec::len);
        if secrets > 0 {
            lines.push(format!("  - mask secrets in {secrets} record(s)"));
        }
        for group in data["topic_groups"].as_array().into_iter().flatten() {
            let names: Vec<_> = group
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            if names.len() > 1 {
                lines.push(format!(
                    "  - merge {} into #{}",
                    names[1..].join(", "),
                    names[0]
                ));
            }
        }
    } else {
        for item in data["applied"].as_array().into_iter().flatten() {
            lines.push(format!("  - {}", item.as_str().unwrap_or("")));
        }
    }
    if lines.last().is_some_and(|line| line.ends_with(':')) {
        lines.push("  (nothing)".into());
    }
    lines.extend([
        String::new(),
        "For you to review (use judgment, then act):".into(),
    ]);
    for (key, label) in [
        ("possible_topics", "Topics that may be the same"),
        ("duplicates", "Entries that may duplicate"),
        ("stale_entries", "Entries whose referenced code moved"),
        ("reported_wrong", "Entries reported wrong"),
        ("doc_candidates", "Busy topics without a doc"),
        ("other_branch_entries", "Entries on another branch"),
        ("stale_docs", "Docs whose code changed"),
        ("idle_plans", "Active plans untouched for 14 days"),
    ] {
        if let Some(items) = data[key].as_array().filter(|items| !items.is_empty()) {
            lines.push(format!("  {label}:"));
            for item in items.iter().take(25) {
                lines.push(format!(
                    "    {}",
                    item["title"]
                        .as_str()
                        .or_else(|| item["topic"].as_str())
                        .unwrap_or_else(|| item.as_str().unwrap_or("see --json"))
                ));
            }
        }
    }
    if lines
        .last()
        .is_some_and(|line| line == "For you to review (use judgment, then act):")
    {
        lines.push("  (nothing)".into());
    }
    lines.join("\n")
}
