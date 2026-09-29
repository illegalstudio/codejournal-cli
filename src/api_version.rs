//! Identifies this release to the server and explains when the server requires a newer one.

use anyhow::{Error, anyhow};
use reqwest::StatusCode;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const HEADER: &str = "X-Cj-Version";

static WARNED: AtomicBool = AtomicBool::new(false);

/// HTTP client that sends the release in `X-Cj-Version` and the user agent.
pub fn client(timeout: Duration) -> reqwest::Result<Client> {
    let mut headers = HeaderMap::new();
    headers.insert(HEADER, HeaderValue::from_static(VERSION));
    Client::builder()
        .timeout(timeout)
        .user_agent(user_agent())
        .default_headers(headers)
        .build()
}

pub fn user_agent() -> String {
    let (os, arch) = (std::env::consts::OS, std::env::consts::ARCH);
    format!("cj/{VERSION} ({os}; {arch})")
}

pub fn upgrade_required(status: StatusCode) -> bool {
    status == StatusCode::UPGRADE_REQUIRED
}

/// The server's upgrade message, or a generic one when the response has none.
pub fn message(value: &Value) -> String {
    value["message"]
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| {
            format!("cj {VERSION} is no longer supported by this server; install a newer release")
        })
}

/// Error for a failed request, with the upgrade message alone when the release is too old.
pub fn error(status: StatusCode, value: &Value) -> Error {
    if upgrade_required(status) {
        anyhow!(message(value))
    } else {
        anyhow!("API returned {status}: {value}")
    }
}

/// Warns once per process when a read falls back to the cache or a write stays queued.
pub fn warn(value: &Value) {
    if !WARNED.swap(true, Ordering::Relaxed) {
        eprintln!("warning: {}", message(value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn upgrade_errors_show_only_the_server_message() {
        let body = json!({"message": "cj 0.1.0 is no longer supported by this server."});
        let upgrade = error(StatusCode::UPGRADE_REQUIRED, &body).to_string();
        assert_eq!(upgrade, "cj 0.1.0 is no longer supported by this server.");
        assert!(message(&json!({})).contains(VERSION));
        let forbidden = error(StatusCode::FORBIDDEN, &json!({"message": "Forbidden"})).to_string();
        assert!(forbidden.starts_with("API returned 403"));
        assert!(user_agent().starts_with(&format!("cj/{VERSION} (")));
    }
}
