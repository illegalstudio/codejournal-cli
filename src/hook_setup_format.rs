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
        } else if dry_run {
            lines.push(format!(
                "Would {} Code Journal hooks in {path}.",
                if install { "install" } else { "remove" }
            ));
        } else if row["changed"] == true {
            lines.push(format!(
                "{} Code Journal hooks for {agent} in {path}.",
                if install { "Installed" } else { "Removed" }
            ));
            if let Some(backup) = row["backup"].as_str() {
                lines.push(format!("Backup of the previous file: {backup}"));
            }
        } else {
            lines.push(format!("Nothing to change in {path}."));
        }
    }
    if install && !status && !dry_run && results.iter().any(|row| row["changed"] == true) {
        lines.push(
            "New sessions pick up the hooks. In Codex, review and trust them with /hooks."
                .to_owned(),
        );
    }
    lines.join("\n")
}
