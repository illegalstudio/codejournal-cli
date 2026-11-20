use crate::{api::Api, attribution, output};
use anyhow::Result;
use serde_json::json;

pub(super) fn run(
    api: &Api,
    tenant: &str,
    id: &str,
    kind: &str,
    reference: &str,
    json_mode: bool,
) -> Result<()> {
    let result = api.post(&format!("{}/use", super::path(tenant, id)),
        &json!({"kind": kind, "ref": reference, "agent": attribution::agent(None), "host": attribution::host()}))?;
    let outcome = if result["recorded"] == true {
        "Recorded usage"
    } else {
        "Usage already recorded"
    };
    let suffix = if result["promoted"] == true {
        " Promoted to durable knowledge."
    } else {
        ""
    };
    output::emit(
        &result,
        &format!("{outcome} for {}.{suffix}", super::short(id)),
        json_mode,
    )
}
