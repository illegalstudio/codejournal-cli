use crate::api::Api;
use crate::{output, project_bootstrap};
use anyhow::{Result, bail};
use clap::Subcommand;
use serde_json::json;

#[derive(Subcommand)]
pub enum RefsAction {
    /// Move repository path refs; OLD and NEW accept an optional path: prefix.
    Move { old: String, new: String },
}

pub fn run(
    api: &Api,
    tenant: &str,
    explicit: Option<&str>,
    action: RefsAction,
    json_mode: bool,
) -> Result<()> {
    let RefsAction::Move { old, new } = action;
    let old = crate::path_ref::normalize(old.trim().strip_prefix("path:").unwrap_or(old.trim()))?;
    let new = crate::path_ref::normalize(new.trim().strip_prefix("path:").unwrap_or(new.trim()))?;
    if old.is_empty() || new.is_empty() || old == new {
        bail!("pass two different paths: cj refs move OLD NEW");
    }
    let slug = project_bootstrap::resolved_slug(api, tenant, explicit)?;
    let result = api.post(
        &format!("/api/v1/tenants/{tenant}/projects/{slug}/refs/move"),
        &json!({"old": old, "new": new}),
    )?;
    output::emit(
        &result,
        &format!(
            "Moved {} path ref(s) from {old} to {new} in {slug}.",
            result["moved"]
        ),
        json_mode,
    )
}
