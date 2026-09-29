//! Responses that defer work instead of failing it: an outdated release (426) or the plan's rate
//! limit (429). Reads fall back to the cache and queueable writes stay queued for `cj sync`.

use crate::api_version::VERSION;
use anyhow::{Error, anyhow};
use reqwest::StatusCode;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};

static WARNED: AtomicBool = AtomicBool::new(false);

pub fn deferred(status: StatusCode) -> bool {
    status == StatusCode::UPGRADE_REQUIRED || status == StatusCode::TOO_MANY_REQUESTS
}

/// The server's explanation, or a generic one when the response has none.
pub fn message(status: StatusCode, value: &Value) -> String {
    if let Some(message) = value["message"].as_str() {
        return message.to_owned();
    }
    if status == StatusCode::TOO_MANY_REQUESTS {
        "the workspace's API rate limit was reached; retry in a minute".to_owned()
    } else {
        format!("cj {VERSION} is no longer supported by this server; install a newer release")
    }
}

/// Error for a failed request, with only the explanation when the work was deferred or the plan
/// locks the project (402).
pub fn error(status: StatusCode, value: &Value) -> Error {
    if deferred(status) || status == StatusCode::PAYMENT_REQUIRED {
        anyhow!(message(status, value))
    } else {
        anyhow!("API returned {status}: {value}")
    }
}

/// Warns once per process when a read falls back to the cache or a write stays queued.
pub fn warn(status: StatusCode, value: &Value) {
    if !WARNED.swap(true, Ordering::Relaxed) {
        eprintln!("warning: {}", message(status, value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn deferred_errors_show_only_the_server_message() {
        let body = json!({"message": "cj 0.1.0 is no longer supported by this server."});
        let upgrade = error(StatusCode::UPGRADE_REQUIRED, &body).to_string();
        assert_eq!(upgrade, "cj 0.1.0 is no longer supported by this server.");
        assert!(message(StatusCode::UPGRADE_REQUIRED, &json!({})).contains(VERSION));
        assert!(message(StatusCode::TOO_MANY_REQUESTS, &json!({})).contains("rate limit"));
        let forbidden = error(StatusCode::FORBIDDEN, &json!({"message": "Forbidden"})).to_string();
        assert!(forbidden.starts_with("API returned 403"));
    }
}
