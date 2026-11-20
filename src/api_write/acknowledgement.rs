use crate::{api::Api, api_cache, request_outbox::PendingRequest};
use anyhow::{Context, Result, ensure};
use serde_json::Value;

pub const FORMAT: &str = "compact-v1";

pub fn accept(api: &Api, request: &PendingRequest, value: &Value) -> Result<Option<Value>> {
    if value["replay"] != FORMAT {
        return Ok(None);
    }
    ensure!(
        value["request_id"] == request.id && value["applied"] == true,
        "invalid compact replay acknowledgement for request {}",
        request.id
    );
    let result = &value["result"];
    ensure!(
        result.is_object(),
        "compact replay is missing its result object"
    );
    let parts = request
        .path
        .trim_matches('/')
        .split('/')
        .collect::<Vec<_>>();
    if matches!(
        (request.method.as_str(), parts.as_slice()),
        ("POST", ["api", "v1", "tenants", _, "projects"])
            | ("PATCH", ["api", "v1", "tenants", _, "projects", _])
    ) {
        ensure!(
            result["project"]["slug"]
                .as_str()
                .is_some_and(|slug| !slug.is_empty()),
            "compact project replay is missing its canonical slug"
        );
    }
    for (collection, resource) in [("plans", "plan"), ("docs", "doc")] {
        if let Some(id) = result[resource]["id"].as_str() {
            ensure!(
                uuid::Uuid::parse_str(id).is_ok(),
                "invalid record ID in compact replay"
            );
            let tenant = parts
                .get(3)
                .context("compact replay is missing its workspace")?;
            let cache = format!("/api/v1/tenants/{tenant}/{collection}/{id}?history=0");
            api_cache::forget(api.server(), &api.token, &cache)?;
        }
    }
    Ok(Some(result.clone()))
}
