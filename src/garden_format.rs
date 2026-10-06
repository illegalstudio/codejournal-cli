use crate::{garden_review_format, git, topic_analysis};
use serde_json::Value;

pub fn render(data: &Value) -> String {
    let dry_run = data["dry_run"] == true;
    let mode = if dry_run {
        "dry run, nothing written"
    } else {
        "safe fixes applied"
    };
    let mut lines = vec![format!(
        "Garden report for {} ({mode})",
        data["project"].as_str().unwrap_or("")
    )];
    if let Some(root) = git::root() {
        let branch = git::output(&["symbolic-ref", "--quiet", "--short", "HEAD"])
            .unwrap_or_else(|| "detached".to_owned());
        lines.push(format!(
            "Path checks use this checkout: {} on branch {branch}. Paths that exist in a cited commit or branch, or on the default branch, count as another branch, not as stale.",
            root.display()
        ));
    }
    lines.push(String::new());
    lines.push(
        if dry_run {
            "Would apply automatically:"
        } else {
            "Applied automatically:"
        }
        .into(),
    );
    let mut applied = Vec::new();
    if dry_run {
        let secrets = data["secrets"].as_array().map_or(0, Vec::len);
        if secrets > 0 {
            applied.push(format!("mask secrets in {secrets} record(s)"));
        }
        for group in data["topic_groups"].as_array().into_iter().flatten() {
            let names = group
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>();
            if names.len() > 1 {
                applied.push(format!(
                    "merge {} into #{}",
                    names[1..]
                        .iter()
                        .map(|name| format!("#{name}"))
                        .collect::<Vec<_>>()
                        .join(", "),
                    names[0]
                ));
            }
        }
    } else {
        applied.extend(
            data["applied"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned),
        );
    }
    if applied.is_empty() {
        lines.push("  (nothing)".into());
    } else {
        lines.extend(applied.into_iter().map(|item| format!("  - {item}")));
    }
    lines.extend([
        String::new(),
        "For you to review (use judgment, then act):".into(),
    ]);
    let before = lines.len();
    garden_review_format::sections(data, &mut lines);
    if lines.len() == before {
        lines.push("  (nothing)".into());
    }
    if let Some(notice) = topic_analysis::notice(data) {
        lines.push(String::new());
        lines.push(notice.to_owned());
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::render;
    use serde_json::json;

    #[test]
    fn report_explains_staleness_and_shows_actionable_ids() {
        let text = render(&json!({"project": "sample", "dry_run": true,
            "stale_entries": [{"id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
                "title": "Old note", "staleness": {"gone": [], "changes": 5}}],
            "reported_wrong": [{"id": "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
                "title": "Wrong note", "wrong": 2}]}));
        assert!(text.contains("aaaaaaaa Old note  [verify: refs changed by 5 commits]"));
        assert!(text.contains("bbbbbbbb Wrong note  [reported wrong 2x]"));
    }
}
