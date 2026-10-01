use crate::api::Api;
use crate::knowledge_args::TopicsAction;
use crate::{output, project};
use anyhow::Result;
use serde_json::{Value, json};

pub fn run(
    api: &Api,
    tenant: &str,
    explicit_project: Option<&str>,
    action: Option<TopicsAction>,
    json_mode: bool,
) -> Result<()> {
    let slug = project::slug(explicit_project)?;
    let path = format!("/api/v1/tenants/{tenant}/projects/{slug}/topics");
    match action {
        None => {
            let result = api.get(&path)?;
            let lines = result["topics"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .map(|item| {
                            format!("  {} ({})", string(&item["name"]), number(&item["count"]))
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            output::emit(
                &result,
                &if lines.is_empty() {
                    "  (no topics yet)".to_owned()
                } else {
                    lines.join("\n")
                },
                json_mode,
            )
        }
        Some(TopicsAction::Merge { sources, into }) => {
            let target = into.trim();
            let sources: Vec<_> = sources
                .iter()
                .flat_map(|value| value.split(','))
                .map(str::trim)
                .filter(|name| !name.is_empty() && *name != target)
                .collect();
            if target.is_empty() || sources.is_empty() {
                anyhow::bail!("pass the topics to fold and --into TARGET");
            }
            let result = api.post(
                &format!("/api/v1/tenants/{tenant}/topics/merge"),
                &json!({"sources": sources, "target": target}),
            )?;
            let joined = result["sources"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(|name| format!("#{name}"))
                .collect::<Vec<_>>()
                .join(", ");
            output::emit(
                &result,
                &format!("Merged {joined} into #{}.", string(&result["target"])),
                json_mode,
            )
        }
        Some(TopicsAction::Similar) => {
            let result = api.get(&format!("/api/v1/tenants/{tenant}/topics/similar"))?;
            let certain: Vec<Vec<String>> = serde_json::from_value(result["certain"].clone())?;
            let possible: Vec<Vec<String>> = serde_json::from_value(result["possible"].clone())?;
            let mut lines = vec![
                "Same topic, different spelling (safe to merge, `cj garden` merges them):"
                    .to_owned(),
            ];
            if certain.is_empty() {
                lines.push("  (none)".to_owned());
            }
            for group in certain.iter().filter(|group| group.len() >= 2) {
                lines.push(format!(
                    "  cj topics merge {} --into {}",
                    group[1..].join(" "),
                    group[0]
                ));
            }
            lines.push("".to_owned());
            lines.push("Possibly the same (check, then merge if so):".to_owned());
            if possible.is_empty() {
                lines.push("  (none)".to_owned());
            }
            for pair in possible.iter().filter(|pair| pair.len() >= 2) {
                lines.push(format!("  {} ~ {}", pair[0], pair[1]));
            }
            output::emit(
                &json!({"certain": certain, "possible": possible}),
                &lines.join("\n"),
                json_mode,
            )
        }
    }
}

fn string(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
fn number(value: &Value) -> u64 {
    value.as_u64().unwrap_or(0)
}
