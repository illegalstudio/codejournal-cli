use crate::api::Api;
use crate::{attribution, output};
use anyhow::{Result, bail};
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
    let compact = id.replace('-', "");
    if !(8..=32).contains(&compact.len()) || !compact.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("invalid plan or doc ID");
    }
    if target != "@global"
        && (target.is_empty()
            || !target
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'))
    {
        bail!("invalid target project slug; use @global for tenant-global docs");
    }
    if api.offline() {
        api.get(&format!("{}?history=0", path(tenant, kind, id)))
            .map_err(|error| error.context("offline moves require a cached, verified source ID; run show --current-only online first"))?;
        if target != "@global" {
            api.get(&format!("/api/v1/tenants/{tenant}/projects/{target}"))
                .map_err(|error| error.context("offline moves require a cached destination project; use @global for tenant-global docs"))?;
        }
    }
    if target == "@global" && kind != "docs" {
        bail!("plans cannot be moved to global scope");
    }
    let result = api
        .patch(
            &path(tenant, kind, id),
            &json!({"action": "move", "to": target,
        "note": note, "agent": attribution::agent(agent.as_deref()), "host": attribution::host()}),
        )
        .map_err(|error| {
            if kind == "docs"
                && target == "global"
                && error
                    .chain()
                    .any(|cause| cause.to_string().contains("Project not found"))
            {
                error.context("use --to @global for a tenant-global doc; global is a project slug")
            } else {
                error
            }
        })?;
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
