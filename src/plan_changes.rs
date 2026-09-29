use crate::api::Api;
use crate::{attribution, output};
use anyhow::Result;
use serde_json::json;

fn noun(kind: &str) -> &str {
    if kind == "docs" { "doc" } else { "plan" }
}
fn path(tenant: &str, kind: &str, id: &str) -> String {
    format!("/api/v1/tenants/{tenant}/{kind}/{id}")
}
fn short(id: &str) -> String {
    id.replace('-', "").chars().take(8).collect()
}

pub fn status(
    api: &Api,
    tenant: &str,
    kind: &str,
    id: &str,
    status: &str,
    note: Option<String>,
    agent: Option<String>,
    json_mode: bool,
) -> Result<()> {
    let note = note.unwrap_or_else(|| format!("status set to {status}"));
    let result = api.patch(
        &path(tenant, kind, id),
        &json!({
            "status": status, "note": note,
            "agent": attribution::agent(agent.as_deref()), "host": attribution::host(),
        }),
    )?;
    let item = &result[noun(kind)];
    output::emit(
        &result,
        &format!(
            "{} {} is now at revision {} ({status}).",
            noun(kind),
            short(item["id"].as_str().unwrap_or(id)),
            item["revision"]
        ),
        json_mode,
    )
}

pub fn move_to(
    api: &Api,
    tenant: &str,
    kind: &str,
    id: &str,
    target: &str,
    note: Option<String>,
    agent: Option<String>,
    json_mode: bool,
) -> Result<()> {
    let result = api.patch(
        &path(tenant, kind, id),
        &json!({"action": "move", "to": target,
        "note": note, "agent": attribution::agent(agent.as_deref()), "host": attribution::host()}),
    )?;
    output::emit(
        &result,
        &format!("Moved {} {} to {target}.", noun(kind), short(id)),
        json_mode,
    )
}

pub fn schedule(
    api: &Api,
    tenant: &str,
    kind: &str,
    id: &str,
    date: &str,
    json_mode: bool,
) -> Result<()> {
    let result = api.patch(&path(tenant, kind, id), &json!({"not_before": date}))?;
    output::emit(
        &result,
        &format!("Scheduled {} {} for {}.", noun(kind), short(id), date),
        json_mode,
    )
}
