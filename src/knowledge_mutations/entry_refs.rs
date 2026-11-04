use crate::{api::Api, output, refs};
use anyhow::{Result, ensure};
use serde_json::json;

pub fn run(
    api: &Api,
    tenant: &str,
    id: &str,
    base: &str,
    raw: Vec<String>,
    json_mode: bool,
) -> Result<()> {
    ensure!(
        base.len() == 64 && base.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "--base must be the refs_version from cj entry show ID --json"
    );
    let result = api.patch(
        &format!("/api/v1/tenants/{tenant}/entries/{id}/refs"),
        &json!({"refs":refs::parse_all(&raw)?,"based_on":base.to_ascii_lowercase()}),
    )?;
    ensure!(
        result["entry"]["refs_version"].is_string(),
        "the server did not confirm protected entry references; inspect the current entry before retrying"
    );
    output::emit(
        &result,
        &format!("Updated references of entry {id}; identity, content and history preserved."),
        json_mode,
    )
}
