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
    let mut body = body;
    if let Some(value) = &mut body {
        output::record_masking(secret_redaction::value(value));
    }
    let mut request = request_outbox::new(&api.server, method, path, body);
    let can_queue = path.starts_with("/api/v1/tenants/");
    if let Some(tenant) = project_bootstrap::tenant_for_path(api, path) {
        let slug = project_bootstrap::ensure(api, &tenant, false)?;
        request.path = project_bootstrap::replace_slug(path, &slug);
    } else if method == "PATCH" || path.contains("/paths") {
        request.path = project_bootstrap::read_path(api, path)?;
    }
    if api.offline {
        if !can_queue {
            bail!("this command is unavailable offline");
        }
        return queued(&request);
    }
    let response = match builder(api, &request)?.send() {
        Ok(response) => response,
        Err(_error) if can_queue => return queued(&request),
        Err(error) => return Err(error.into()),
    };
    let mut status = response.status();
    if status.is_server_error() && can_queue {
        return queued(&request);
    }
    let mut value: Value = match response.json() {
        Ok(value) => crate::api::sanitized(value),
        Err(_error) if can_queue && status.is_success() => return queued(&request),
        Err(error) => return Err(error.into()),
    };
    // An outdated release or the rate limit keeps the write for replay by `cj sync`.
    if api_status::deferred(status) && can_queue {
        api_status::warn(status, &value);
        return queued(&request);
    }
    if status == reqwest::StatusCode::NOT_FOUND
        && value["message"] == "Project not found"
        && let Some(tenant) = project_bootstrap::tenant_for_path(api, path)
    {
        let slug = project_bootstrap::ensure(api, &tenant, true)?;
        request.path = project_bootstrap::replace_slug(path, &slug);
        let retry = builder(api, &request)?.send()?;
        status = retry.status();
        value = crate::api::sanitized(retry.json()?);
    }
    if !status.is_success() {
        return Err(api_status::error(status, &value));
    }
    Ok(value)
}

pub fn replay(api: &Api, request: &PendingRequest) -> Result<Value> {
    if api.offline {
        bail!("cannot replay while offline");
    }
    let response = builder(api, request)?.send()?;
    let status = response.status();
    let value: Value = crate::api::sanitized(response.json()?);
    if !status.is_success() {
        return Err(api_status::error(status, &value));
    }
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
    if let Some(body) = &request.body {
        builder = builder.json(body);
    }
    Ok(builder)
}

fn queued(request: &PendingRequest) -> Result<Value> {
    request_outbox::enqueue(request)?;
    Err(QueuedWrite(request.id.clone()).into())
}
