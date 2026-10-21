use super::{Scope, state};
use crate::{api::Api, config::Config, outbox, request_age, request_outbox};
use anyhow::Result;
use serde_json::{Value, json};

pub fn inspect(config: Option<&Config>, server: Option<&str>) -> Result<Value> {
    let requests = request_outbox::entries()?;
    let mut hook_origin = None;
    let mut result = json!({"enabled": super::enabled(), "running": false,
        "pending_current_scope": null, "pending_other_scopes": null,
        "last_attempt_at": null, "last_sync_at": null, "next_retry_at": null, "last_error": null, "blocked": false});
    if let Some(config) = config
        && let Ok(token) = config.token_for(server.unwrap_or(&config.server))
    {
        let api = Api::new(server.unwrap_or(&config.server), &token)?;
        let scope = Scope::new(&api, &config.tenant);
        let saved = state::load(&scope)?;
        result["running"] = json!(state::try_lock(&scope)?.is_none());
        result["last_attempt_at"] = json!(saved.last_attempt_at);
        result["last_sync_at"] = json!(saved.last_sync_at);
        result["next_retry_at"] = json!(saved.next_retry_at);
        result["last_error"] = json!(saved.last_error);
        result["blocked"] = json!(saved.blocked);
        let writes = scope.pending_writes()?;
        result["pending_current_scope"] = json!(writes);
        result["pending_other_scopes"] = json!(requests.len().saturating_sub(writes));
        hook_origin = Some(scope.origin);
    }
    let hooks = outbox::diagnostics::inspect(hook_origin.as_deref())?;
    if hook_origin.is_some() {
        let current = result["pending_current_scope"].as_u64().unwrap_or(0);
        result["pending_current_scope"] = json!(
            current
                + if hooks.delivery_enabled {
                    hooks.current_scope.unwrap_or(0) as u64
                } else {
                    0
                }
        );
        result["pending_other_scopes"] = json!(
            result["pending_other_scopes"].as_u64().unwrap_or(0)
                + hooks.other_scopes.unwrap_or(0) as u64
        );
    }
    let oldest = requests
        .iter()
        .filter_map(|(_, request)| request.created_at)
        .chain(hooks.oldest_pending_at)
        .min();
    result["oldest_pending_at"] = json!(oldest);
    result["oldest_pending_age_seconds"] =
        json!(oldest.map(|time| request_age::now().saturating_sub(time)));
    result["hook_notices"] = json!(outbox::diagnostics::notices(&hooks));
    result["hook_events"] = json!(hooks);
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
        lines.push(format!("other scope: {count} writes/events preserved; inspect cj outbox list and cj outbox hooks list and use their original account/server"));
    }
    if let Some(notices) = value["hook_notices"].as_array() {
        lines.extend(notices.iter().filter_map(Value::as_str).map(str::to_owned));
    }
    lines.join("\n")
}

pub fn synchronized(api: &Api, tenant: &str) -> Result<()> {
    state::synchronized(api, tenant)
}
