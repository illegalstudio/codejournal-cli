use crate::{api::Api, request_outbox::PendingRequest};
use anyhow::Result;
use reqwest::Method;

pub fn builder(api: &Api, request: &PendingRequest) -> Result<reqwest::blocking::RequestBuilder> {
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
