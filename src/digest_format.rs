use serde_json::Value;

pub fn render(data: &Value, label: &str) -> String {
    let mut lines = vec![format!("# Code Journal digest: {label}"), String::new()];
    let Some(projects) = data["projects"].as_object() else {
        return format!("{}Nothing happened in this window.\n", lines.join("\n"));
    };
    let feedback = data["feedback"].as_array();
    if projects.is_empty() && feedback.is_none_or(Vec::is_empty) {
        lines.push("Nothing happened in this window.".into());
        return lines.join("\n") + "\n";
    }
    let mut sorted = projects.iter().collect::<Vec<_>>();
    sorted.sort_by_key(|(_, bucket)| std::cmp::Reverse(items(bucket, "logs").len()));
    for (slug, bucket) in sorted {
        lines.extend([format!("## {slug}"), String::new()]);
        group(
            &mut lines,
            "Waiting for you",
            items(bucket, "needs_you"),
            |row| {
                format!(
                    "{} ({}, {})",
                    text(&row["title"]),
                    row["agent"].as_str().unwrap_or("agent"),
                    date(&row["created_at"])
                )
            },
        );
        group(&mut lines, "Errors", items(bucket, "errors"), |row| {
            text(&row["title"]).to_owned()
        });
        let logs = items(bucket, "logs");
        group(
            &mut lines,
            &format!(
                "Work ({} log{}):",
                logs.len(),
                if logs.len() == 1 { "" } else { "s" }
            ),
            logs,
            |row| {
                format!(
                    "[{}] {} ({})",
                    text(&row["status"]),
                    text(&row["title"]),
                    row["agent"].as_str().unwrap_or("agent")
                )
            },
        );
        for (key, heading) in [("plans", "Plans updated"), ("docs", "Docs updated")] {
            group(&mut lines, heading, items(bucket, key), |row| {
                format!(
                    "{} ({}, rev {})",
                    text(&row["title"]),
                    text(&row["status"]),
                    row["revision"]
                )
            });
        }
        let entries = items(bucket, "entries");
        if !entries.is_empty() {
            lines.push(format!("**New knowledge ({} entries):**", entries.len()));
            for row in entries.iter().rev().take(5).rev() {
                lines.push(format!("- {}", text(&row["title"])));
            }
            lines.push(String::new());
        }
    }
    if let Some(feedback) = feedback.filter(|items| !items.is_empty()) {
        lines.extend(["## Feedback on Code Journal".into(), String::new()]);
        for row in feedback {
            lines.push(format!(
                "- [{}] {}",
                text(&row["category"]),
                text(&row["title"])
            ));
        }
    }
    lines.join("\n").trim_end().to_owned() + "\n"
}

fn group(
    lines: &mut Vec<String>,
    heading: &str,
    rows: &[Value],
    format: impl Fn(&Value) -> String,
) {
    if rows.is_empty() {
        return;
    }
    lines.push(format!("**{}**", heading.trim_end_matches(':')));
    for row in rows {
        lines.push(format!("- {}", format(row)));
    }
    lines.push(String::new());
}

fn items<'a>(bucket: &'a Value, key: &str) -> &'a [Value] {
    bucket[key].as_array().map(Vec::as_slice).unwrap_or(&[])
}
fn text(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
fn date(value: &Value) -> &str {
    let text = text(value);
    &text[..text.len().min(16)]
}
