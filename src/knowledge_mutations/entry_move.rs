use crate::{api::Api, attribution, output};
use anyhow::{Result, bail};
use serde_json::json;

pub fn run(
    api: &Api,
    tenant: &str,
    id: &str,
    to: &str,
    old: Option<String>,
    new: Option<String>,
    dry_run: bool,
    json_mode: bool,
) -> Result<()> {
    if dry_run && api.offline() {
        bail!("entry move --dry-run needs an online connection and is never queued");
    }
    let path = format!("/api/v1/tenants/{tenant}/entries/{id}/move");
    let body = json!({"to": to, "path_prefix": old, "replace_prefix": new, "dry_run": dry_run,
        "agent": attribution::agent(None), "host": attribution::host()});
    let result = if dry_run {
        api.post_noqueue(&path, &body)?
    } else {
        api.post(&path, &body)?
    };
    output::emit(
        &result,
        &format!(
            "{} entry {id} from {} to {to}.",
            if dry_run { "Would move" } else { "Moved" },
            result["from"].as_str().unwrap_or("unknown")
        ),
        json_mode,
    )
}
