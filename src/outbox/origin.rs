use crate::config::Config;
use serde_json::{Value, json};

pub fn capture(event: &mut Value) {
    let config = Config::load();
    let binding = config
        .as_ref()
        .map_err(|_| "configuration_unavailable")
        .and_then(|config| {
            let server = std::env::var("CJ_SERVER_URL").unwrap_or(config.server.clone());
            config
                .token_for(&server)
                .map(|token| super::fingerprint(&server, &config.tenant, &token))
                .map_err(|_| "credentials_unavailable")
        });
    match binding {
        Ok(origin) => {
            event["origin"] = json!(origin);
        }
        Err(reason) => {
            event["origin"] = Value::Null;
            event["origin_unavailable_reason"] = json!(reason);
        }
    }
}

pub fn bound(event: &Value) -> Option<&str> {
    event["origin"].as_str().filter(|value| !value.is_empty())
}
