use crate::output;
use anyhow::Result;
use serde_json::{Value, json};

const LIMIT: usize = 100_000;
const WARN_AT: usize = 90_000;

pub fn emit(result: &Value, text: &str, json_mode: bool) -> Result<()> {
    let mut result = result.clone();
    let mut text = text.to_owned();
    if let Some((noun, characters)) = ["plan", "doc"].into_iter().find_map(|noun| {
        result[noun]["body"]
            .as_str()
            .map(|body| (noun, body.chars().count()))
    }) {
        let remaining = LIMIT.saturating_sub(characters);
        result["body_size"] = json!({"characters": characters, "limit": LIMIT,
            "remaining": remaining, "warning_at": WARN_AT});
        let mut metadata =
            format!("Body size: {characters}/{LIMIT} characters ({remaining} remaining).");
        let historical = result["current_revision"].as_u64().is_some_and(|current| {
            result[noun]["revision"]
                .as_u64()
                .is_some_and(|selected| selected != current)
        });
        if characters >= WARN_AT && !historical {
            let warning = format!(
                "This {noun} is approaching the {LIMIT}-character body limit. Compact the current state before appending more progress; existing numbered revisions remain available through cj {noun} show ID --history."
            );
            if !result["warnings"].is_array() {
                result["warnings"] = json!([]);
            }
            if let Some(warnings) = result["warnings"].as_array_mut() {
                warnings.push(json!(warning));
            }
            metadata.push_str(&format!("\n{warning}"));
        }
        text = format!("{metadata}\n{text}");
    }
    output::emit(&result, &text, json_mode)
}
