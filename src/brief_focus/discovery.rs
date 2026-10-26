use serde_json::{Value, json};

/// Older servers and offline caches must follow the same discovery policy.
pub fn apply(data: &mut Value, active: bool, summary: bool) {
    for (section, statuses) in [
        ("plans", &["active"][..]),
        ("docs", &["current"][..]),
        ("global_docs", &["current"][..]),
        ("tasks", &["open", "in_progress"][..]),
        ("pinned", &["active"][..]),
        ("recent", &["active"][..]),
        ("entries", &["active"][..]),
        ("questions", &["active"][..]),
        ("global", &["active"][..]),
    ] {
        if active && let Some(rows) = data[section].as_array_mut() {
            rows.retain(|row| {
                row["status"]
                    .as_str()
                    .is_some_and(|status| statuses.contains(&status))
            });
        }
    }
    if active {
        if data.get("forwarded_results").is_some() {
            data["forwarded_results"] = json!([]);
        }
        if let Some(rows) = data["delegations"].as_array_mut() {
            rows.retain(|row| {
                !["done", "failed", "cancelled", "stalled"]
                    .contains(&row["status"].as_str().unwrap_or(""))
            });
        }
        if let Some(focus) = data.get_mut("focus") {
            apply(focus, true, summary);
        }
        let ids: Vec<_> = ["plans", "docs", "global_docs", "tasks"]
            .into_iter()
            .flat_map(|section| data[section].as_array().into_iter().flatten())
            .filter_map(|row| row["id"].as_str().map(str::to_owned))
            .collect();
        if let Some(rows) = data["now_due"].as_array_mut() {
            rows.retain(|row| {
                row["id"]
                    .as_str()
                    .is_some_and(|id| ids.iter().any(|known| known == id))
            });
        }
    }
    if !summary {
        return;
    }
    if let Some(project) = data.get_mut("project") {
        *project = fields(project, &["id", "slug", "name", "remote_url"]);
    }
    for section in [
        "plans",
        "docs",
        "global_docs",
        "tasks",
        "logs",
        "pinned",
        "recent",
        "entries",
        "questions",
        "global",
        "forwarded_results",
        "delegations",
    ] {
        if let Some(rows) = data[section].as_array_mut() {
            for row in rows {
                *row = fields(
                    row,
                    &[
                        "id",
                        "project_id",
                        "project_slug",
                        "source_project_id",
                        "plan_id",
                        "kind",
                        "scope",
                        "title",
                        "task_title",
                        "target",
                        "status",
                        "priority",
                        "revision",
                        "not_before",
                        "updated_at",
                        "created_at",
                        "agent",
                        "refs",
                        "topics",
                        "usage",
                        "staleness",
                    ],
                );
            }
        }
    }
    if !active && let Some(focus) = data.get_mut("focus") {
        apply(focus, false, true);
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_discovery_keeps_rules_logs_and_active_records() {
        let mut data = json!({"rules":"Complete rules", "project":{"slug":"test", "rules":"Duplicate"},
            "plans":[{"id":"a", "status":"active", "body":"Private"}, {"id":"d", "status":"draft"}],
            "docs":[{"id":"c", "status":"current"}, {"id":"o", "status":"outdated"}],
            "tasks":[{"status":"in_progress"}, {"status":"done"}],
            "logs":[{"status":"done", "title":"Recent work", "body":"Private"}],
            "now_due":[{"id":"a"},{"id":"d"}], "forwarded_results":[{"status":"done"}],
            "focus":{"docs":[{"status":"outdated"}, {"status":"current", "body":"Private"}]}});
        apply(&mut data, true, true);
        assert_eq!(data["rules"], "Complete rules");
        for section in ["plans", "docs", "tasks", "logs", "now_due"] {
            assert_eq!(data[section].as_array().unwrap().len(), 1);
        }
        assert_eq!(data["focus"]["docs"].as_array().unwrap().len(), 1);
        assert!(!data.to_string().contains("Private"));
        assert_eq!(data["forwarded_results"], json!([]));
    }

    #[test]
    fn explicit_all_and_verbose_preserve_inactive_content() {
        let mut data = json!({"docs":[{"status":"outdated", "body":"Requested"}], "focus":{"docs":[{"status":"draft"}]}});
        let original = data.clone();
        apply(&mut data, false, false);
        assert_eq!(data, original);
        apply(&mut data, false, true);
        assert_eq!(data["docs"][0]["status"], "outdated");
        assert!(data["docs"][0].get("body").is_none());
    }
}
