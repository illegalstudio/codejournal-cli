use serde_json::Value;

pub fn render(data: &Value) -> String {
    let counts = &data["counts"];
    let mut lines = vec![
        format!(
            "Garden reviews for {}{}",
            value(&data["project"]),
            if data["dry_run"] == true {
                " (preview, nothing written)"
            } else {
                ""
            }
        ),
        format!(
            "Pending: {} | Reviewed: {} | Deferred: {}",
            counts["pending"], counts["reviewed"], counts["deferred"]
        ),
        format!(
            "Last scan: {} | Last review: {}",
            value(&counts["last_scan"]),
            value(&counts["last_review"])
        ),
    ];
    for action in data["applied"].as_array().into_iter().flatten() {
        lines.push(format!("Applied: {}", value(action)));
    }
    if data["automatic"]["secrets"].as_u64().unwrap_or(0) > 0
        || data["automatic"]["topic_groups"].as_u64().unwrap_or(0) > 0
    {
        lines.push(format!(
            "Safe repairs available: {} secret records, {} equivalent topic groups",
            data["automatic"]["secrets"], data["automatic"]["topic_groups"]
        ));
    }
    for row in data["findings"].as_array().into_iter().flatten() {
        lines.push(format!(
            "  {}  [{}] {}",
            short(&row["id"]),
            value(&row["kind"]),
            value(&row["title"])
        ));
        for target in row["targets"].as_array().into_iter().flatten() {
            lines.push(format!(
                "    {} {}",
                value(&target["kind"]),
                value(&target["id"])
            ));
        }
        if row["details"]["changes"].as_u64().unwrap_or(0) > 0 {
            lines.push(format!(
                "    {} commits touched its refs; verify the content",
                row["details"]["changes"]
            ));
        }
        if let Some(paths) = row["details"]["gone"]
            .as_array()
            .filter(|paths| !paths.is_empty())
        {
            lines.push(format!(
                "    Missing refs ({}): {}",
                row["details"]["gone_count"]
                    .as_u64()
                    .unwrap_or(paths.len() as u64),
                paths.iter().map(value).collect::<Vec<_>>().join(", ")
            ));
        }
    }
    if data["findings"].as_array().is_none_or(Vec::is_empty) {
        lines.push("  No pending findings in this batch.".into());
    }
    if let Some(next) = data["next"].as_str() {
        lines.push(format!(
            "Continue: cj garden --after {next} | Complete output: cj garden --all"
        ));
    }
    if data["dry_run"] == true
        && data["next"].is_null()
        && counts["pending"].as_u64().unwrap_or(0)
            > data["findings"].as_array().map_or(0, Vec::len) as u64
    {
        lines.push("Complete preview: cj garden --dry-run --all. Run cj garden to persist a resumable queue.".into());
    }
    lines.push("After verifying or fixing one finding: cj garden review ID --outcome verified --note \"evidence\". Use corrected, superseded, obsolete, dismissed or deferred when appropriate.".into());
    if data["partial"] == true {
        lines.push("Topic analysis is partial; other findings remain reviewable. Explicit topic merges remain available with cj topics merge.".into());
    }
    if data["code_scan_partial"] == true {
        lines.push(format!("Code scan incomplete: {} records could not be read; their previous findings remain pending.", data["snapshot_failure_count"]));
        for failure in data["scan_failures"].as_array().into_iter().flatten() {
            lines.push(format!(
                "  {} {}: {}",
                value(&failure["kind"]),
                value(&failure["id"]),
                value(&failure["error"])
            ));
        }
    }
    if data["cached"] == true {
        lines.push(
            "Cached review queue: counts and findings may be outdated; record outcomes online."
                .into(),
        );
    }
    lines.join("\n")
}

fn value(value: &Value) -> &str {
    value.as_str().unwrap_or("never")
}
fn short(value: &Value) -> String {
    value
        .as_str()
        .unwrap_or("")
        .replace('-', "")
        .chars()
        .take(8)
        .collect()
}
