use crate::api::public_client;
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
        .unwrap_or_else(|| "https://codejournal-saas.ddev.site".to_owned());
    let server = server.trim_end_matches('/').to_owned();
    let client = public_client(&server)?;
    let response = client
        .post(format!("{server}/api/v1/device/requests"))
        .header("Accept", "application/json")
        .send()?;
    if !response.status().is_success() {
        bail!("could not start login: {}", response.text()?);
    }
    let request: DeviceRequest = response.json()?;
    let url = format!("{}?code={}", request.verification_uri, request.user_code);
    println!("Open {url}\nConfirm code: {}", request.user_code);
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
        if !response.status().is_success() {
            bail!("login failed: {}", response.text()?);
        }
        let payload: Value = response.json()?;
        let token = payload["access_token"]
            .as_str()
            .context("missing access token")?;
        let profile: Value = client
            .get(format!("{server}/api/v1/me"))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .send()?
            .json()?;
        let tenant = profile["tenant"]["slug"]
            .as_str()
            .context("missing tenant")?;
        Config::store(server, tenant.to_owned(), token.to_owned())?;
        println!("Logged in to workspace {tenant}.");
        return Ok(());
    }
    bail!("device code expired; run cj login again")
}
