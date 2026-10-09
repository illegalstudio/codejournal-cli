use anyhow::{Context, Result};
use reqwest::blocking::Response;
use serde_json::Value;

pub fn checked(response: Response) -> Result<Value> {
    let status = response.status();
    let delay = super::response_error::retry_after(response.headers());
    let value = decode(response)
        .map_err(|error| super::response_error::ResponseError::wrap(status, delay, error))?;
    if !status.is_success() {
        return Err(super::response_error::ResponseError::wrap(
            status,
            delay,
            crate::api_status::error(status, &value),
        ));
    }
    Ok(value)
}

pub fn at(response: Response, endpoint: &str) -> Result<Value> {
    let status = response.status();
    let delay = super::response_error::retry_after(response.headers());
    let endpoint = endpoint.split('?').next().unwrap_or(endpoint);
    let (endpoint, _) = crate::secret_redaction::text(endpoint);
    checked(response).map_err(|error| {
        let guidance = if status == reqwest::StatusCode::NOT_FOUND {
            "; check cj status for the server/workspace and cj projects for registration; the server may not support this endpoint"
        } else { "" };
        super::response_error::ResponseError::wrap(status, delay,
            anyhow::anyhow!("{error} ({endpoint}){guidance}"))
    })
}

pub fn decode(response: Response) -> Result<Value> {
    let status = response.status();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|header| header.to_str().ok())
        .unwrap_or("missing")
        .split(';')
        .next()
        .unwrap_or("missing")
        .chars()
        .take(80)
        .filter(|character| character.is_ascii_alphanumeric() || "/+.-".contains(*character))
        .collect::<String>();
    let (content_type, _) = crate::secret_redaction::text(&content_type);
    response.json().map(super::sanitized).with_context(|| {
        format!("API returned HTTP {status}, Content-Type {content_type}, with invalid JSON")
    })
}
