use crate::{
    api::Api,
    api_status, output, project_bootstrap,
    request_outbox::{self, PendingRequest, QueuedWrite},
    secret_redaction,
};
use anyhow::{Result, bail};
use reqwest::Method;
use serde_json::Value;

pub fn mutate(api: &Api, method: &str, path: &str, body: Option<Value>) -> Result<Value> {
    crate::request_ids::reject_pending(api, path, body.as_ref())?;
    let mut body = body;
    if let Some(value) = &mut body {
        output::record_masking(secret_redaction::value(value));
    }
    let mut request = request_outbox::new(&api.server, method, path, body);
    let can_queue = path.starts_with("/api/v1/tenants/");
    if let Some(tenant) = project_bootstrap::tenant_for_path(api, path) {
        let slug = project_bootstrap::ensure(api, &tenant, false).map_err(|error| {
            error.context(format!(
                "write not queued (request {}): project preparation failed",
                request.id
            ))
        })?;
        request.path = project_bootstrap::replace_slug(path, &slug);
    } else if method == "PATCH" || path.contains("/paths") {
        request.path = project_bootstrap::read_path(api, path).map_err(|error| {
            error.context(format!(
                "write not queued (request {}): project resolution failed",
                request.id
            ))
        })?;
    }
    if api.offline {
        if !can_queue {
            bail!("this command is unavailable offline");
        }
        return queued(&request);
    }
    let (mut status, mut value) = attempt(api, &request, can_queue)?;
    if status == reqwest::StatusCode::NOT_FOUND
        && value["message"] == "Project not found"
        && let Some(tenant) = project_bootstrap::tenant_for_path(api, path)
    {
        let slug = project_bootstrap::ensure(api, &tenant, true).map_err(|error| {
            error.context(format!(
                "write not queued (request {}): project preparation failed",
                request.id
            ))
        })?;
        request.path = project_bootstrap::replace_slug(path, &slug);
        (status, value) = attempt(api, &request, can_queue)?;
    }
    if !status.is_success() {
        return Err(api_status::error(status, &value)
            .context(format!("write not queued (request {})", request.id)));
    }
    crate::project_cache::updated(api, &request, &value)?;
    Ok(value)
}

fn attempt(
    api: &Api,
    request: &PendingRequest,
    can_queue: bool,
) -> Result<(reqwest::StatusCode, Value)> {
    let response = match builder(api, request)?.send() {
        Ok(response) => response,
        Err(_) if can_queue => return queued(request),
        Err(error) => return Err(error.into()),
    };
    let status = response.status();
    if can_queue && status.is_server_error() {
        return queued(request);
    }
    if can_queue && api_status::deferred(status) {
        let value = response
            .json::<Value>()
            .map(crate::api::sanitized)
            .unwrap_or_default();
        api_status::warn(status, &value);
        return queued(request);
    }
    let value = match response.json() {
        Ok(value) => crate::api::sanitized(value),
        Err(_) if can_queue && status.is_success() => return queued(request),
        Err(_) => bail!(
            "write not queued (request {}): API returned {status} with invalid JSON",
            request.id
        ),
    };
    Ok((status, value))
}

pub fn replay(api: &Api, request: &PendingRequest) -> Result<Value> {
    crate::request_age::ensure_retryable(request.created_at)?;
    if api.offline {
        bail!("cannot replay while offline");
    }
    let response = builder(api, request)?.send()?;
    let status = response.status();
    let value: Value = crate::api::sanitized(response.json()?);
    if !status.is_success() {
        return Err(api_status::error(status, &value));
    }
    crate::project_cache::updated(api, request, &value)?;
    Ok(value)
}

fn builder(api: &Api, request: &PendingRequest) -> Result<reqwest::blocking::RequestBuilder> {
    let method = Method::from_bytes(request.method.as_bytes())?;
    let mut builder = api
        .client
        .request(method, format!("{}{}", api.server, request.path))
        .bearer_auth(&api.token)
        .header("Accept", "application/json")
        .header("Idempotency-Key", &request.id);
    if let Some(seconds) = request.created_at {
        builder = builder.header("Idempotency-Created-At", seconds.to_string());
    }
    if let Some(body) = &request.body {
        builder = builder.json(body);
    }
    Ok(builder)
}

fn queued<T>(request: &PendingRequest) -> Result<T> {
    request_outbox::enqueue(request).map_err(|error| {
        error.context(format!(
            "write could not be queued (request {})",
            request.id
        ))
    })?;
    Err(QueuedWrite(request.id.clone()).into())
}
