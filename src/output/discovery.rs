use serde_json::Value;

/// Keep list discovery concise and lifecycle-scoped even with older servers.
pub fn list(data: &mut Value, section: &str, status: &str, verbose: bool) {
    let Some(rows) = data[section].as_array_mut() else {
        return;
    };
    rows.retain(|row| match (section, status) {
        (_, "all") => true,
        ("tasks", "active") => matches!(row["status"].as_str(), Some("open" | "in_progress")),
        ("tasks", "closed") => matches!(row["status"].as_str(), Some("done" | "dismissed")),
        ("plans", "open") => matches!(row["status"].as_str(), Some("draft" | "active")),
        ("docs", "open") => matches!(
            row["status"].as_str(),
            Some("draft" | "current" | "outdated")
        ),
        ("watches", "active") => matches!(row["status"].as_str(), Some("starting" | "running")),
        ("notifications", "unread") => row["read_at"].is_null(),
        ("notifications", "read") => !row["read_at"].is_null(),
        _ => row["status"] == status,
    });
    if verbose {
        return;
    }
    for row in rows {
        if let Some(fields) = row.as_object_mut() {
            fields.retain(|key, _| {
                matches!(
                    key.as_str(),
                    "id" | "project_id"
                        | "project_slug"
                        | "source_project_id"
                        | "source_project_slug"
                        | "source_slug"
                        | "forwarded"
                        | "plan_id"
                        | "kind"
                        | "scope"
                        | "title"
                        | "status"
                        | "priority"
                        | "revision"
                        | "not_before"
                        | "updated_at"
                        | "created_at"
                        | "agent"
                        | "refs"
                        | "topics"
                        | "match_excerpt"
                        | "usage"
                        | "staleness"
                        | "category"
                        | "read_at"
                        | "source"
                        | "host"
                        | "started_at"
                        | "ended_at"
                        | "exit_code"
                        | "runner_id"
                        | "lease_expires_at"
                )
            });
        }
    }
}
