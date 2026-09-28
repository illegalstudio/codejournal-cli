use crate::{
    api::Api,
    request_outbox::{self, PendingRequest, QueuedWrite},
};
use anyhow::{Result, bail};
use reqwest::Method;
use serde_json::Value;

pub fn mutate(api: &Api, method: &str, path: &str, body: Option<Value>) -> Result<Value> {
    let request = request_outbox::new(&api.server, method, path, body);
    let can_queue = path.starts_with("/api/v1/tenants/");
    if api.offline {
        if !can_queue {
            bail!("this command is unavailable offline");
        }
        return queued(&request);
    }
    let response = builder(api, &request)?.send();
    let response = match response {
        Ok(response) => response,
        Err(_error) if can_queue => return queued(&request),
        Err(error) => return Err(error.into()),
    };
    let status = response.status();
    if status.is_server_error() && can_queue {
        return queued(&request);
    }
    let value: Value = match response.json() {
        Ok(value) => value,
        Err(_error) if can_queue && status.is_success() => return queued(&request),
        Err(error) => return Err(error.into()),
    };
    if !status.is_success() {
        bail!("API returned {status}: {value}");
    }
    Ok(value)
}

pub fn replay(api: &Api, request: &PendingRequest) -> Result<Value> {
    if api.offline {
        bail!("cannot replay while offline");
    }
    let response = builder(api, request)?.send()?;
    let status = response.status();
    let value: Value = response.json()?;
    if !status.is_success() {
        bail!("API returned {status}: {value}");
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
