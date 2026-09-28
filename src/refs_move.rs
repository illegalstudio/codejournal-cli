use crate::api::Api;
use crate::{output, project};
use anyhow::{Result, bail};
use clap::Subcommand;
use serde_json::json;

#[derive(Subcommand)]
pub enum RefsAction {
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
    let old = old.trim_matches('/');
    let new = new.trim_matches('/');
    if old.is_empty() || new.is_empty() || old == new {
        bail!("pass two different paths: cj refs move OLD NEW");
    }
    let slug = project::slug(explicit)?;
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
