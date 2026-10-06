use crate::{
    api::{Api, sanitized},
    api_cache, api_status, project_bootstrap,
};
use anyhow::{Context, Result, bail};
use serde_json::Value;

pub fn fallback(api: &Api, path: &str, cause: &str) -> Result<Value> {
    match api_cache::read(api.server(), &api.token, path) {
        Ok(mut value) => {
            eprintln!("{cause}; using a cached response, which may be outdated.");
            if value.is_object() {
                value["cached"] = Value::Bool(true);
            }
            Ok(sanitized(value))
        }
        Err(_) => bail!(
            "{cause}. No cached response is available for this request. Cached briefs may still be available with cj brief --offline. Retry online when the service is available; pending writes can be inspected with cj outbox list"
        ),
    }
}

pub fn get(api: &Api, path: &str) -> Result<Value> {
    crate::request_ids::reject_pending(api, path, None)?;
    let path = project_bootstrap::read_path(api, path)?;
    if api.offline() {
        return fallback(api, &path, "Offline mode");
    }
    let response = match api
        .client
        .get(format!("{}{}", api.server(), path))
        .bearer_auth(&api.token)
        .header("Accept", "application/json")
        .send()
    {
        Ok(response) => response,
        Err(_) => return fallback(api, &path, "The Code Journal server could not be reached"),
    };
    let status = response.status();
    if status.is_server_error() {
        return fallback(
            api,
            &path,
            &format!("The Code Journal server returned {status}"),
        );
    }
    let value = sanitized(response.json().context("API returned invalid JSON")?);
    if api_status::deferred(status) {
        api_status::warn(status, &value);
        return fallback(api, &path, &api_status::message(status, &value));
    }
    if !status.is_success() {
        return Err(api_status::error(status, &value));
    }
    let _ = api_cache::write(api.server(), &api.token, &path, &value);
    Ok(value)
}
