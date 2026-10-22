use crate::{api::Api, attribution};
use anyhow::{Context, Result};
use serde_json::json;

#[allow(clippy::too_many_arguments)]
pub fn run(
    api: &Api,
    tenant: &str,
    kind: &str,
    id: &str,
    number: u32,
    done: bool,
    note: Option<String>,
    json_mode: bool,
) -> Result<()> {
    let path = format!("/api/v1/tenants/{tenant}/{kind}/{id}");
    let noun = if kind == "docs" { "doc" } else { "plan" };
    let read = format!("{path}?history=0");
    let current = if api.offline() {
        api.get(&read)?
    } else {
        api.get_fresh(&read).context(
            "could not read a fresh checklist revision; nothing was sent or queued. Retry online, or use --offline explicitly to edit cached content",
        )?
    };
    crate::plan_checklist::step(
        current[noun]["body"]
            .as_str()
            .context("missing checklist body")?,
        u64::from(number),
        done,
    )?;
    let revision = current[noun]["revision"]
        .as_u64()
        .context("missing current revision")?;
    let result = api.patch(
        &path,
        &json!({"action": "step", "step_index": number,
        "step_done": done, "based_on": revision, "note": note,
        "agent": attribution::agent(None), "host": attribution::host()}),
    )?;
    crate::plan_show::size::emit(
        &result,
        &format!("Updated checklist item {number} of {noun} {id}."),
        json_mode,
    )
}
