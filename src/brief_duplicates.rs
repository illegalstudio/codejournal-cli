use serde_json::Value;

/// Warns that another project holds a journal for this same checkout path.
pub fn lines(data: &Value) -> Vec<String> {
    data["duplicate_checkouts"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|item| {
            let slug = text(&item["slug"]);
            format!(
                "Duplicate journal: {}:{} is also registered under project {slug} ({} active entries, {} work logs), whose history is not shown here. Read it with `cj brief --project {slug}` and tell the user. Combining them needs the user's confirmation (`cj project merge SOURCE --into TARGET`); a stale registration is removed with `cj project path-remove PATH --project {slug}`.",
                text(&item["host"]),
                text(&item["path"]),
                item["active_entries"].as_u64().unwrap_or(0),
                item["logs"].as_u64().unwrap_or(0),
            )
        })
        .collect()
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
