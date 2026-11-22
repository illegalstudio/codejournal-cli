//! Markdown for a pull request or issue as `gh ... --json` returns it.
use serde_json::Value;

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}

fn remarks(lines: &mut Vec<String>, heading: &str, items: &Value, date: &str) {
    let items: Vec<_> = items
        .as_array()
        .into_iter()
        .flatten()
        .filter(|item| !text(&item["body"]).trim().is_empty() || !text(&item["state"]).is_empty())
        .collect();
    if items.is_empty() {
        return;
    }
    lines.push(format!("\n## {heading}"));
    for item in items {
        let state = text(&item["state"]);
        lines.push(format!(
            "\n### {} at {}{}\n\n{}",
            text(&item["author"]["login"]),
            text(&item[date]),
            if state.is_empty() {
                String::new()
            } else {
                format!(" ({state})")
            },
            text(&item["body"]).trim()
        ));
    }
}

pub fn render(record: &Value, diff: Option<&str>, diff_limit: usize) -> String {
    let kind = if diff.is_some() {
        "Pull request"
    } else {
        "Issue"
    };
    let mut lines = vec![
        format!("# {kind} #{}: {}", record["number"], text(&record["title"])),
        String::new(),
        format!("URL: {}", text(&record["url"])),
        format!(
            "State: {} | Author: {}",
            text(&record["state"]),
            text(&record["author"]["login"])
        ),
    ];
    if diff.is_some() {
        lines.push(format!(
            "Branches: {} into {}",
            text(&record["headRefName"]),
            text(&record["baseRefName"])
        ));
    }
    lines.push(format!(
        "\n## Description\n\n{}",
        text(&record["body"]).trim()
    ));
    remarks(&mut lines, "Comments", &record["comments"], "createdAt");
    remarks(&mut lines, "Reviews", &record["reviews"], "submittedAt");
    if let Some(diff) = diff {
        let mut end = diff.len().min(diff_limit);
        while !diff.is_char_boundary(end) {
            end -= 1;
        }
        lines.push(format!("\n## Diff\n\n{}", &diff[..end]));
        if end < diff.len() {
            lines.push(format!(
                "\n[Diff cut at {end} of {} bytes to fit one attachment; ask the host for the rest.]",
                diff.len()
            ));
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::render;
    use serde_json::json;

    #[test]
    fn a_pull_request_keeps_its_discussion_and_says_when_the_diff_is_cut() {
        let record = json!({"number": 7, "title": "Fix lowering", "state": "OPEN", "url": "https://example.test/pull/7",
            "author": {"login": "alice"}, "body": "Why it changed", "headRefName": "fix", "baseRefName": "main",
            "comments": [{"author": {"login": "bob"}, "createdAt": "2026-11-21T10:00:00Z", "body": "Please test"}],
            "reviews": [{"author": {"login": "carol"}, "submittedAt": "2026-11-21T11:00:00Z", "state": "APPROVED", "body": ""}]});
        let text = render(&record, Some("diff --git a/é b/é\n+line\n"), 14);
        assert!(text.starts_with("# Pull request #7: Fix lowering"));
        assert!(
            text.contains("Branches: fix into main")
                && text.contains("### bob at 2026-11-21T10:00:00Z")
        );
        assert!(text.contains("### carol at 2026-11-21T11:00:00Z (APPROVED)"));
        assert!(text.contains("[Diff cut at 13 of 27 bytes"));
        let issue = render(&record, None, 14);
        assert!(
            issue.starts_with("# Issue #7")
                && !issue.contains("## Diff")
                && !issue.contains("Branches")
        );
    }
}
