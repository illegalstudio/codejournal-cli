use super::format;
use crate::output;
use anyhow::Result;
use serde_json::{Value, json};

pub fn emit(
    response: Value,
    params: &[(&str, String)],
    all_projects: bool,
    json_mode: bool,
) -> Result<()> {
    let entries = response["entries"].as_array().cloned().unwrap_or_default();
    let lines: Vec<_> = entries
        .iter()
        .map(|entry| {
            let mut line = format::line(entry, all_projects);
            if params
                .iter()
                .any(|(key, value)| *key == "summary" && value == "0")
            {
                for detail in entry["body"].as_str().unwrap_or("").lines() {
                    line.push_str(&format!("\n      {detail}"));
                }
            }
            line
        })
        .collect();
    let mut notices = super::empty::notices(params, entries.is_empty());
    if let Some((_, query)) = params.iter().find(|(key, _)| *key == "q")
        && !query.contains(char::is_whitespace)
        && !params
            .iter()
            .any(|(key, value)| *key == "whole_word" && value == "1")
        && !entries.is_empty()
    {
        let pattern = regex::Regex::new(&format!(
            r"(?i)(?:^|[^\pL\pN_]){}(?:$|[^\pL\pN_])",
            regex::escape(query)
        ))?;
        if entries.iter().all(|entry| {
            entry["match_excerpt"].is_string()
                && !pattern.is_match(&format!(
                    "{} {}",
                    entry["title"].as_str().unwrap_or(""),
                    entry["match_excerpt"].as_str().unwrap_or("")
                ))
        }) {
            notices.push("Results may include stem or substring matches. Retry with --whole-word for exact word boundaries, or --topic to narrow the subsystem.".to_owned());
        }
    }
    let payload = json!({
        "project": response["project"], "entries": entries, "cached": response["cached"] == true, "notices": notices,
    });
    output::emit(
        &payload,
        &if lines.is_empty() {
            std::iter::once("No matching entries.".to_owned())
                .chain(notices)
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            lines
                .into_iter()
                .chain(notices)
                .collect::<Vec<_>>()
                .join("\n")
        },
        json_mode,
    )
}
