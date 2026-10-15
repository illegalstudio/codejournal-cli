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
    let notices = super::empty::notices(params, entries.is_empty());
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
            lines.join("\n")
        },
        json_mode,
    )
}
