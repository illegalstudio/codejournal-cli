use crate::{api::Api, output};
use anyhow::Result;
use serde_json::json;

pub fn change(
    api: &Api,
    tenant: &str,
    kind: &str,
    id: &str,
    action: &str,
    json_mode: bool,
) -> Result<()> {
    let result = api.patch(
        &format!("/api/v1/tenants/{tenant}/{kind}/{id}/retention"),
        &json!({"action": action}),
    )?;
    let line = if let Some(date) = result["purge_at"].as_str() {
        format!("Moved {id} to trash. Scheduled deletion: {date}.")
    } else {
        format!("Restored {id} from trash.")
    };
    output::emit(&result, &line, json_mode)
}
