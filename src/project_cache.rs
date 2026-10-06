use crate::{api::Api, api_cache, request_outbox::PendingRequest};
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::collections::HashSet;

pub fn canonical(api: &Api, tenant: &str, slug: &str) -> Result<String> {
    let mut current = slug.to_owned();
    let mut seen = HashSet::new();
    while seen.insert(current.clone()) {
        let key = format!("/local-slug/{tenant}/{current}");
        let Ok(value) = api_cache::read(api.server(), &api.token, &key) else {
            return Ok(current);
        };
        let Some(next) = value["slug"].as_str() else {
            return Ok(current);
        };
        if next == current {
            return Ok(current);
        }
        current = next.to_owned();
    }
    bail!("cached project rename cycle; resolve the project using an explicit --project slug")
}

pub fn updated(api: &Api, request: &PendingRequest, response: &Value) -> Result<()> {
    let parts = request
        .path
        .trim_matches('/')
        .split('/')
        .collect::<Vec<_>>();
    if request.method != "PATCH" || parts.len() != 6 || parts[4] != "projects" {
        return Ok(());
    }
    if let Some(new) = response["project"]["slug"].as_str() {
        let current = format!("/local-slug/{}/{new}", parts[3]);
        api_cache::write(api.server(), &api.token, &current, &json!({"slug": new}))?;
        let key = format!("/local-slug/{}/{}", parts[3], parts[5]);
        api_cache::write(api.server(), &api.token, &key, &json!({"slug": new}))?;
    }
    Ok(())
}

pub fn cache_created(
    api: &Api,
    request: &PendingRequest,
    response: &serde_json::Value,
) -> Result<Option<(String, String)>> {
    let parts = request
        .path
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() != 5 || parts[0..3] != ["api", "v1", "tenants"] || parts[4] != "projects" {
        return Ok(None);
    }
    let Some(body) = request.body.as_ref() else {
        return Ok(None);
    };
    let Some(old) = body["slug"].as_str() else {
        return Ok(None);
    };
    let Some(new) = response["project"]["slug"].as_str() else {
        return Ok(None);
    };
    if let Some(path) = body["path"].as_str() {
        let key = crate::project_bootstrap::cache_key(parts[3], path, body["remote_url"].as_str());
        api_cache::write(api.server(), &api.token, &key, &json!({"slug": new}))?;
    }
    Ok(Some((
        format!("{}/{}", request.path, old),
        format!("{}/{}", request.path, new),
    )))
}
