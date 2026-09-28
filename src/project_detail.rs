use serde_json::Value;
use std::path::Path;

pub fn format(project: &Value) -> String {
    let provides = &project["provides"];
    let auto = names(&provides["auto"]);
    let manual = names(&provides["manual"]);
    let mut all = auto.clone();
    for name in &manual {
        if !all.contains(name) {
            all.push(name.clone());
        }
    }
    if let Some(remote) = project["remote_url"].as_str() {
        let parts = remote.split('/').collect::<Vec<_>>();
        all.push(remote.to_owned());
        if let Some(name) = parts.last() {
            all.push((*name).to_owned());
        }
        if parts.len() >= 3 {
            all.push(parts[parts.len() - 2..].join("/"));
        }
    }
    all.retain(|name| !name.is_empty());
    all = all
        .into_iter()
        .map(|name| name.to_ascii_lowercase())
        .collect();
    all.sort();
    all.dedup();
    let counts = project["counts"]
        .as_object()
        .map(|counts| {
            let mut rows = counts.iter().collect::<Vec<_>>();
            rows.sort_by_key(|(status, _)| *status);
            rows.into_iter()
                .map(|(status, count)| format!("{} {status}", count.as_u64().unwrap_or(0)))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "none".to_owned());
    let mut lines = vec![
        format!("slug:            {}", value(&project["slug"])),
        format!("name:            {}", value(&project["name"])),
        format!(
            "remote:          {}",
            optional(&project["remote_url"], "(none)")
        ),
        format!("id:              {}", value(&project["id"])),
        format!("created:         {}", value(&project["created_at"])),
        format!(
            "rules updated:   {}",
            optional(&project["rules_updated_at"], "never")
        ),
        format!(
            "provides:        {}{}",
            if all.is_empty() {
                "(none)".to_owned()
            } else {
                all.join(", ")
            },
            if manual.is_empty() {
                String::new()
            } else {
                format!("  (manual: {})", manual.join(", "))
            }
        ),
        format!("entries:         {counts}"),
        "checkouts:".to_owned(),
    ];
    for path in project["paths"].as_array().into_iter().flatten() {
        let kind = value(&path["kind"]);
        let label = if kind == "main" {
            kind.to_owned()
        } else {
            format!("{kind}:{}", optional(&path["branch"], "detached"))
        };
        let location = value(&path["path"]);
        let missing = if value(&path["host"]) == crate::attribution::host()
            && !Path::new(location).is_dir()
        {
            "  (missing)"
        } else {
            ""
        };
        lines.push(format!(
            "  {label:<24} {:<10} {location}{missing}",
            value(&path["host"])
        ));
    }
    lines.join("\n")
}

fn names(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item.as_str().map(str::to_owned))
        .collect()
}

fn optional<'a>(value: &'a Value, fallback: &'a str) -> &'a str {
    value.as_str().unwrap_or(fallback)
}

fn value(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::format;
    use serde_json::json;

    #[test]
    fn detail_includes_python_project_fields() {
        let project = json!({
            "slug": "team-app", "name": "App", "remote_url": null, "id": "p1",
            "created_at": "2026-01-01", "rules_updated_at": null,
            "provides": {"auto": ["app"], "manual": ["app", "cli"]},
            "counts": {"active": 2, "obsolete": 1}, "paths": [{
                "kind": "worktree", "branch": "feature/x", "host": "another-host",
                "path": "/somewhere/worktree"
            }]
        });
        let text = format(&project);
        assert!(text.contains("provides:        app, cli  (manual: app, cli)"));
        assert!(text.contains("entries:         2 active, 1 obsolete"));
        assert!(text.contains("worktree:feature/x"));
    }
}
