use crate::api::Api;
use crate::output;
use anyhow::{Result, bail};
use serde_json::json;

pub(super) fn run(
    api: &Api,
    tenant: &str,
    id: &str,
    wrong: bool,
    note: Option<String>,
    agent: Option<String>,
    json_mode: bool,
) -> Result<()> {
    if wrong && note.as_deref().is_none_or(str::is_empty) {
        bail!("say what is wrong with --note, then supersede or obsolete the entry if you can");
    }
    let kind = if wrong { "wrong" } else { "helpful" };
    let result = api.post(
        &format!("{}/flags", super::path(tenant, id)),
        &json!({"kind": kind, "note": note, "agent": agent}),
    )?;
    let mut human = format!("Flagged {} as {kind}.", super::short(id));
    if wrong {
        human.push_str(" Supersede it with the correct fact (`cj add`, then `cj supersede`) or mark it obsolete.");
    }
    output::emit(&result, &human, json_mode)
}
