use crate::{
    api::{Api, sanitized},
    api_cache, api_status, project_bootstrap,
};
use anyhow::Result;
use serde_json::Value;

/// A read that reached neither the server nor a cached copy of the same request.
#[derive(Debug)]
pub struct NoCachedResponse(String);

impl std::fmt::Display for NoCachedResponse {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for NoCachedResponse {}

pub fn fallback(api: &Api, path: &str, cause: &str) -> Result<Value> {
    match api_cache::read(api.server(), &api.token, path) {
        Ok(mut value) => {
            let saved = api_cache::saved_at(api.server(), &api.token, path);
            eprintln!(
                "{cause}; using a response cached {}, which may be outdated.",
                saved.as_deref().map_or("earlier".to_owned(), |at| format!("at {at}"))
            );
            if value.is_object() {
                value["cached"] = Value::Bool(true);
                value["cached_at"] = serde_json::json!(saved);
            }
            Ok(sanitized(value))
        }
        Err(_) => Err(NoCachedResponse(format!(
            "{cause}. No cached response is available for this request; project context is unavailable, not empty. Cached briefs may still be available with cj brief --offline. Work can continue locally and durable writes can be queued with --offline; queued writes are not synchronized. Inspect them with cj outbox list and retry online when the service is available"
        ))
        .into()),
    }
}

pub fn get(api: &Api, path: &str) -> Result<Value> {
    crate::request_ids::reject_pending(api, path, None)?;
    if api.offline()
        && let Some(value) = crate::plan_write::pending::read(api, path)?
    {
        return Ok(value);
    }
    let path = project_bootstrap::read_path(api, path)?;
    if api.offline() {
        return fallback(api, &path, "Offline mode");
    }
    let response = match api
        .client
        .get(format!("{}{}", api.server(), path))
        .bearer_auth(&api.token)
        .header("Accept", "application/json")
        .timeout(std::time::Duration::from_secs(
            if path.contains("/export") { 20 } else { 5 },
        ))
        .send()
    {
        Ok(response) => response,
        Err(error) => {
            return fallback(
                api,
                &path,
                &format!(
                    "The Code Journal server could not be reached (GET {}{}){}",
                    api.server(),
                    path.split('?').next().unwrap_or(&path),
                    if error.is_timeout() {
                        ": read timed out"
                    } else {
                        ""
                    }
                ),
            );
        }
    };
    let status = response.status();
    let value = match crate::api::response::at(response, &format!("GET {}{path}", api.server())) {
        Ok(value) => value,
        Err(error)
            if status.is_server_error()
                || status.is_success()
                || api_status::deferred(status)
                || status == reqwest::StatusCode::REQUEST_TIMEOUT =>
        {
            if api_status::deferred(status) {
                eprintln!("warning: {error}");
            }
            return fallback(api, &path, &error.to_string());
        }
        Err(error) => return Err(error),
    };
    let _ = api_cache::write(api.server(), &api.token, &path, &value);
    Ok(value)
}
