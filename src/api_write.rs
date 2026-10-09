use crate::api::queue::{queued, queued_after};
use crate::{
    api::Api,
    api_status, output, project_bootstrap,
    request_outbox::{self, PendingRequest},
    secret_redaction,
};
use anyhow::{Result, bail};
use serde_json::Value;

mod request;

pub fn mutate(api: &Api, method: &str, path: &str, body: Option<Value>) -> Result<Value> {
    crate::request_ids::reject_pending(api, path, body.as_ref())?;
    let mut body = body;
    if let Some(value) = &mut body {
        output::record_masking(secret_redaction::value(value));
    }
    let mut request = request_outbox::new(&api.server, method, path, body);
    if !api.token.is_empty()
        && let Some(tenant) = path.split('/').nth(4)
    {
        request.origin = Some(crate::outbox::fingerprint(api.server(), tenant, &api.token));
    }
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
    if api.offline || (can_queue && has_prior_request(api, &request)?) {
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
    crate::plan_write::pending::remember(
        api,
        request.path.split('/').nth(4).unwrap_or(""),
        &value,
    )?;
    Ok(value)
}

fn attempt(
    api: &Api,
    request: &PendingRequest,
    can_queue: bool,
) -> Result<(reqwest::StatusCode, Value)> {
    let response = match request::builder(api, request)?.send() {
        Ok(response) => response,
        Err(_) if can_queue => return queued(request),
        Err(error) => return Err(error.into()),
    };
    let status = response.status();
    if can_queue && status.is_server_error() {
        return queued_after(
            request,
            crate::api::response_error::retry_after(response.headers()),
        );
    }
    if can_queue && api_status::deferred(status) {
        let delay = crate::api::response_error::retry_after(response.headers());
        let value = response
            .json::<Value>()
            .map(crate::api::sanitized)
            .unwrap_or_default();
        api_status::warn(status, &value);
        return queued_after(request, delay);
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
    let response = request::builder(api, request)?.send()?;
    let value = crate::api::response::at(
        response,
        &format!("{} {}{}", request.method, api.server(), request.path),
    )?;
    crate::project_cache::updated(api, request, &value)?;
    crate::plan_write::pending::remember(
        api,
        request.path.split('/').nth(4).unwrap_or(""),
        &value,
    )?;
    if request.path.ends_with("/checkout-activity")
        && let Some(body) = &request.body
    {
        crate::activity_state::DeliveryState::lock(api.server(), &api.token, &request.path, body)?
            .acknowledge()?;
    }
    Ok(value)
}

fn has_prior_request(api: &Api, request: &PendingRequest) -> Result<bool> {
    let tenant = request.path.split('/').nth(4).unwrap_or("");
    let origin = request.origin.as_deref().unwrap_or("");
    Ok(request_outbox::entries()?
        .iter()
        .any(|(_, prior)| request_outbox::delivery::matches(prior, api.server(), tenant, origin)))
}
