use crate::{api::Api, output, refs};
use anyhow::Result;
use serde_json::json;

pub fn run(api: &Api, tenant: &str, id: &str, values: Vec<String>, json_mode: bool) -> Result<()> {
    let references = refs::parse_all(&values)?;
    let result = api.patch(
        &format!("/api/v1/tenants/{tenant}/logs/{id}"),
        &json!({"refs": references}),
    )?;
    output::emit(
        &result,
        &format!("Updated refs of work log {id}."),
        json_mode,
    )
}
