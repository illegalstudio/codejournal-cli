use crate::api::public_client;
use crate::api_status;
use crate::config::Config;
use anyhow::{Context, Result, bail};
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::{Value, json};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Deserialize)]
struct DeviceRequest {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

pub fn run(explicit_server: Option<&str>) -> Result<()> {
    let server = explicit_server
        .map(str::to_owned)
        .or_else(|| Config::load().ok().map(|config| config.server))
        .unwrap_or_else(|| "https://codejournal.online".to_owned());
    let server = server.trim_end_matches('/').to_owned();
    let client = public_client(&server)?;
    let response = client
        .post(format!("{server}/api/v1/device/requests"))
        .header("Accept", "application/json")
        .send()?;
    let request: DeviceRequest = serde_json::from_value(payload(response)?)?;
    let url = format!("{}?code={}", request.verification_uri, request.user_code);
    crate::stdout::println!("Open {url}\nConfirm code: {}", request.user_code);
    let _ = webbrowser::open(&url);
    let deadline = Instant::now() + Duration::from_secs(request.expires_in);
    while Instant::now() < deadline {
        thread::sleep(Duration::from_secs(request.interval.max(5)));
        let response = client
            .post(format!("{server}/api/v1/device/token"))
            .header("Accept", "application/json")
            .json(&json!({ "device_code": request.device_code }))
            .send()?;
        if response.status() == StatusCode::PRECONDITION_REQUIRED {
            continue;
        }
        let token_response = payload(response)?;
        let token = token_response["access_token"]
            .as_str()
            .context("missing access token")?;
        let response = client
            .get(format!("{server}/api/v1/me"))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .send()?;
        let profile = payload(response)?;
        let tenant = profile["tenant"]["slug"]
            .as_str()
            .context("missing tenant")?;
        Config::store(server, tenant.to_owned(), token.to_owned())?;
        crate::stdout::println!("Logged in to workspace {tenant}.");
        return Ok(());
    }
    bail!("device code expired; run cj login again")
}

fn payload(response: reqwest::blocking::Response) -> Result<Value> {
    let status = response.status();
    let value = response.json().unwrap_or(Value::Null);
    checked_payload(status, value)
}

fn checked_payload(status: StatusCode, value: Value) -> Result<Value> {
    if !status.is_success() {
        return Err(api_status::error(status, &crate::api::sanitized(value)));
    }
    if value.is_null() {
        bail!("API returned invalid JSON during login");
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_are_preserved_but_errors_are_redacted_and_actionable() {
        let token = format!("ghp_{}", "a".repeat(36));
        let body = json!({"access_token": token});
        assert_eq!(checked_payload(StatusCode::OK, body.clone()).unwrap(), body);
        let error = checked_payload(
            StatusCode::UPGRADE_REQUIRED,
            json!({
                "message": format!("Update required {token}"), "minimum_version": "1.0.0",
            }),
        )
        .unwrap_err()
        .to_string();
        assert!(!error.contains(&token));
        assert!(error.contains("cj update"));
    }
}
