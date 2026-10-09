use crate::{api::Api, output};
use anyhow::{Result, bail};
use serde_json::json;

pub fn run(
    api: &Api,
    tenant: &str,
    id: &str,
    target: &str,
    dry_run: bool,
    json_mode: bool,
) -> Result<()> {
    if dry_run && api.offline() {
        bail!("log move --dry-run needs an online connection and is never queued");
    }
    let compact = id.replace('-', "");
    if !(8..=32).contains(&compact.len()) || !compact.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("invalid log ID");
    }
    if target.is_empty()
        || !target
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        bail!("invalid target project slug");
    }
    let path = format!("/api/v1/tenants/{tenant}/logs/{id}/move");
    let input = json!({"to": target, "dry_run": dry_run});
    let result = if dry_run {
        api.post_noqueue(&path, &input)?
    } else {
        api.post(&path, &input)?
    };
    let verb = if dry_run { "Would move" } else { "Moved" };
    output::emit(
        &result,
        &format!("{verb} work log {id} to {target}."),
        json_mode,
    )
}
