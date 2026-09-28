use serde_json::Value;

pub fn render(results: &[Value], status: bool, install: bool, dry_run: bool) -> String {
    let mut lines = Vec::new();
    for row in results {
        let agent = row["agent"].as_str().unwrap_or("");
        let path = row["settings"].as_str().unwrap_or("");
        let label = if agent == "claude" {
            "Claude Code (also run by Cursor)"
        } else {
            "Codex"
        };
        if status {
            lines.push(format!("{label}: {path}"));
            if let Some(error) = row["error"].as_str() {
                lines.push(format!("  error: {error}"));
                continue;
            }
            let events = row["installed"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>();
            lines.push(if events.is_empty() {
                "  Code Journal hooks: not installed (run `cj hooks install`)".to_owned()
            } else {
                format!("  Code Journal hooks: {}", events.join(", "))
            });
            if !events.is_empty() {
                let missing = row["missing"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>();
                if !missing.is_empty() {
                    lines.push(format!(
                        "  missing: {} (run `cj hooks install` again)",
                        missing.join(", ")
                    ));
                }
                let seen = row["sessions_seen"].as_u64().unwrap_or(0);
                let cursor = row["cursor_sessions_seen"].as_u64().unwrap_or(0);
                let cursor = if agent == "claude" {
                    format!(" (+{cursor} from Cursor)")
                } else {
                    String::new()
                };
                let last = row["last_event_at"].as_str().unwrap_or("never");
                lines.push(format!(
                    "  sessions seen on this machine (last 2 days): {seen}{cursor}, last event {last}"
                ));
                if agent == "codex" && seen == 0 {
                    lines.push("  Codex runs new hooks only after you trust them: open Codex, run /hooks, review and trust the Code Journal hooks".to_owned());
                }
            }
        } else if dry_run {
            lines.push(format!(
                "Would {} Code Journal hooks in {path}:",
                if install { "install" } else { "remove" }
            ));
            if install {
                lines.push(serde_json::to_string_pretty(&row["preview"]).unwrap_or_default());
            } else {
                let removed = row["removed"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>();
                lines.push(format!(
                    "  events: {}",
                    if removed.is_empty() {
                        "none installed".to_owned()
                    } else {
                        removed.join(", ")
                    }
                ));
            }
        } else if row["changed"] == true {
            let events = if install {
                &row["installed"]
            } else {
                &row["removed"]
            };
            let events = events
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!(
                "{} Code Journal hooks for {agent} in {path} ({events}).",
                if install { "Installed" } else { "Removed" }
            ));
            if let Some(backup) = row["backup"].as_str() {
                lines.push(format!("Backup of the previous file: {backup}"));
            }
        } else {
            lines.push(format!("Nothing to change in {path}."));
        }
    }
    if status
        && results
            .first()
            .is_some_and(|row| row["queued_events"].as_u64().unwrap_or(0) > 0)
    {
        lines.push(format!(
            "Events waiting to be flushed: {}",
            results[0]["queued_events"]
        ));
    }
    if status && std::env::var("CODE_JOURNAL_HOOKS").as_deref() == Ok("off") {
        lines.push("CODE_JOURNAL_HOOKS=off in this environment: handlers do nothing".to_owned());
    }
    if install && !status && !dry_run && results.iter().any(|row| row["changed"] == true) {
        lines.push(
            "New sessions pick up the hooks. In Codex, review and trust them with /hooks."
                .to_owned(),
        );
    }
    lines.join("\n")
}
