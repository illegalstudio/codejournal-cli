use crate::{api::Api, attribution, output};
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
    let current = api.get(&format!("{path}?history=0"))?;
    let revision = current[noun]["revision"]
        .as_u64()
        .context("missing current revision")?;
    let result = api.patch(
        &path,
        &json!({"action": "step", "step_index": number,
        "step_done": done, "based_on": revision, "note": note,
        "agent": attribution::agent(None), "host": attribution::host()}),
    )?;
    output::emit(
        &result,
        &format!("Updated checklist item {number} of {noun} {id}."),
        json_mode,
    )
}
