use super::{Scope, state};
use crate::{api::Api, config::Config, request_age, request_outbox};
use anyhow::Result;
use serde_json::{Value, json};

pub fn inspect(config: Option<&Config>) -> Result<Value> {
    let requests = request_outbox::entries()?;
    let oldest = requests
        .iter()
        .filter_map(|(_, request)| request.created_at)
        .min();
    let mut result = json!({"enabled": super::enabled(), "running": false,
        "oldest_pending_at": oldest, "oldest_pending_age_seconds": oldest.map(|time| request_age::now().saturating_sub(time)),
        "last_attempt_at": null, "last_sync_at": null, "next_retry_at": null, "last_error": null, "blocked": false});
    if let Some(config) = config
        && let Ok(token) = config.token_for(&config.server)
    {
        let api = Api::new(&config.server, &token)?;
        let scope = Scope::new(&api, &config.tenant);
        let saved = state::load(&scope)?;
        result["running"] = json!(state::try_lock(&scope)?.is_none());
        result["last_attempt_at"] = json!(saved.last_attempt_at);
        result["last_sync_at"] = json!(saved.last_sync_at);
        result["next_retry_at"] = json!(saved.next_retry_at);
        result["last_error"] = json!(saved.last_error);
        result["blocked"] = json!(saved.blocked);
        result["pending_current_scope"] = json!(scope.pending()?);
        result["pending_other_scopes"] =
            json!(requests.len().saturating_sub(scope.pending_writes()?));
    }
    Ok(result)
}

pub fn describe(value: &Value) -> String {
    let mut lines = vec![format!(
        "auto sync:   {}",
        if value["enabled"] == false {
            "disabled"
        } else if value["running"] == true {
            "running"
        } else if value["blocked"] == true {
            "needs attention (cj outbox list; cj sync)"
        } else {
            "idle"
        }
    )];
    if let Some(seconds) = value["oldest_pending_age_seconds"].as_u64() {
        lines.push(format!("oldest:      {seconds}s pending"));
    }
    if let Some(time) = value["next_retry_at"].as_u64() {
        lines.push(format!(
            "next retry:  in {}s",
            time.saturating_sub(request_age::now())
        ));
    }
    if let Some(error) = value["last_error"].as_str() {
        lines.push(format!("sync error:  {error}"));
    }
    if let Some(count) = value["pending_other_scopes"]
        .as_u64()
        .filter(|count| *count > 0)
    {
        lines.push(format!("other scope: {count} writes preserved; inspect cj outbox list and use their original account/server"));
    }
    lines.join("\n")
}

pub fn synchronized(api: &Api, tenant: &str) -> Result<()> {
    state::synchronized(api, tenant)
}
