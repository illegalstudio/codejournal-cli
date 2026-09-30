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
    if let Some(message) = value["message"]
        .as_str()
        .or_else(|| value["error"].as_str())
    {
        return message.to_owned();
    }
    if status == StatusCode::TOO_MANY_REQUESTS {
        "the workspace's API rate limit was reached; retry in a minute".to_owned()
    } else if status == StatusCode::UPGRADE_REQUIRED {
        format!("cj {VERSION} is no longer supported by this server; install a newer release")
    } else {
        status
            .canonical_reason()
            .unwrap_or("request failed")
            .to_owned()
    }
}

/// Error for a failed request, with only the explanation when the work was deferred or the plan
/// locks the project (402).
pub fn error(status: StatusCode, value: &Value) -> Error {
    if deferred(status) || status == StatusCode::PAYMENT_REQUIRED {
        anyhow!(message(status, value))
    } else {
        let mut details = Vec::new();
        if let Some(errors) = value["errors"].as_object() {
            for (field, messages) in errors {
                for text in messages
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                {
                    details.push(format!("{field}: {text}"));
                }
            }
        }
        let mut explanation = message(status, value);
        if status == StatusCode::NOT_FOUND && explanation == "Project not found" {
            explanation
                .push_str("; check --project or register this checkout with cj project init");
        }
        if details.is_empty() {
            anyhow!("API returned {status}: {explanation}")
        } else {
            anyhow!(
                "API returned {status}: {explanation}\n{}",
                details.join("\n")
            )
        }
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

    #[test]
    fn validation_errors_preserve_field_details_without_debug_payloads() {
        let body = json!({"message": "Invalid status", "errors": {"status": ["Use done"]},
            "exception": "PrivateClass", "file": "/private/server.php", "trace": [{"secret": "hidden"}]});
        let error = error(StatusCode::UNPROCESSABLE_ENTITY, &body).to_string();
        assert!(error.contains("Invalid status"));
        assert!(error.contains("status: Use done"));
        for forbidden in ["PrivateClass", "/private", "trace", "secret", "hidden"] {
            assert!(!error.contains(forbidden));
        }
        assert!(
            super::error(StatusCode::NOT_FOUND, &json!({"trace": []}))
                .to_string()
                .ends_with("Not Found")
        );
    }
}
