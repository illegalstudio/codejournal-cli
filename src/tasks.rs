use crate::api::Api;
use crate::task_args::TaskAction;
use crate::{output, project, refs};
use anyhow::{Context, Result, bail};
use serde_json::json;

pub fn run(api: &Api, tenant: &str, project_name: Option<&str>, action: TaskAction) -> Result<()> {
    let source = project::slug(project_name)?;
    let endpoint = |slug: &str| format!("/api/v1/tenants/{tenant}/projects/{slug}/tasks");
    match action {
        TaskAction::List { all } => output::json(&api.get(&format!(
            "{}{}",
            endpoint(&source),
            if all { "?all=1" } else { "" }
        ))?),
        TaskAction::Add {
            title,
            body,
            priority,
            refs: raw_refs,
            not_before,
            target,
            from_entry,
        } => {
            if from_entry.is_some() && target.is_none() {
                bail!("--from-entry requires --to");
            }
            let destination = target.as_deref().unwrap_or(&source);
            output::json(&api.post(&endpoint(destination), &json!({
                "title": title, "body": body, "priority": priority,
                "refs": refs::parse_all(&raw_refs)?, "not_before": not_before,
                "source_project": target.as_ref().map(|_| source), "source_entry_id": from_entry,
            }))?)
        }
        TaskAction::Edit { id, not_before } => mutate(
            api,
            &endpoint(&source),
            &id,
            json!({"not_before": not_before}),
        ),
        TaskAction::Start { id } => {
            mutate(api, &endpoint(&source), &id, json!({"action": "start"}))
        }
        TaskAction::Done { id, note } => mutate(
            api,
            &endpoint(&source),
            &id,
            json!({"action": "done", "note": note}),
        ),
        TaskAction::Dismiss { id, note } => mutate(
            api,
            &endpoint(&source),
            &id,
            json!({"action": "dismiss", "note": note}),
        ),
        TaskAction::Reopen { id } => {
            mutate(api, &endpoint(&source), &id, json!({"action": "reopen"}))
        }
    }
}

fn mutate(api: &Api, endpoint: &str, prefix: &str, body: serde_json::Value) -> Result<()> {
    let listing = api.get(&format!("{endpoint}?all=1"))?;
    let rows = listing["tasks"].as_array().context("invalid task list")?;
    let matches: Vec<&str> = rows
        .iter()
        .filter_map(|row| row["id"].as_str())
        .filter(|id| id.replace('-', "").starts_with(&prefix.replace('-', "")))
        .collect();
    let [id] = matches.as_slice() else {
        bail!("task ID is missing or ambiguous: {prefix}")
    };
    output::json(&api.patch(&format!("{endpoint}/{id}"), &body)?)
}
