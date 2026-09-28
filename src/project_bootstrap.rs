use crate::api_cache;
use crate::request_outbox::{self, PendingRequest, QueuedWrite};
use crate::{api::Api, attribution, checkout_identity, project, project_provides};
use anyhow::{Result, bail};
use serde_json::json;

pub fn tenant_for_path(api: &Api, path: &str) -> Option<String> {
    let parts = matching_parts(api, path)?;
    (parts.len() >= 7 && parts[6] != "paths").then(|| parts[3].to_owned())
}

fn matching_parts<'a>(api: &Api, path: &'a str) -> Option<Vec<&'a str>> {
    if !api.auto_project {
        return None;
    }
    let parts = path
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() < 6 || parts[0..3] != ["api", "v1", "tenants"] || parts[4] != "projects" {
        return None;
    }
    (project::slug(None).ok()?.as_str() == parts[5]).then_some(parts)
}

pub fn read_path(api: &Api, path: &str) -> Result<String> {
    let Some(parts) = matching_parts(api, path) else {
        return Ok(path.to_owned());
    };
    let tenant = parts[3];
    let checkout =
        checkout_identity::current().ok_or_else(|| anyhow::anyhow!("Git checkout missing"))?;
    let key = cache_key(
        tenant,
        &checkout.root.to_string_lossy(),
        checkout.origin.as_deref(),
    );
    if let Ok(cached) = api_cache::read(api.server(), &api.token, &key)
        && let Some(slug) = cached["slug"].as_str()
    {
        return Ok(replace_slug(path, slug));
    }
    if api.offline() {
        bail!("no cached project identity for this checkout");
    }
    let query = reqwest::Url::parse_with_params(
        "http://local/",
        &[
            ("remote_url", checkout.origin.as_deref().unwrap_or("")),
            ("host", attribution::host().as_str()),
            ("path", checkout.root.to_string_lossy().as_ref()),
        ],
    )?;
    let endpoint = format!(
        "/api/v1/tenants/{tenant}/projects/resolve?{}",
        query.query().unwrap_or("")
    );
    let found = api.get(&endpoint)?;
    let canonical = found["project"]["slug"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("project resolver returned no slug"))?;
    api_cache::write(api.server(), &api.token, &key, &json!({"slug": canonical}))?;
    Ok(replace_slug(path, canonical))
}

pub fn ensure(api: &Api, tenant: &str, force: bool) -> Result<String> {
    let slug = project::slug(None)?;
    let base = format!("/api/v1/tenants/{tenant}/projects");
    let checkout = checkout_identity::current();
    let key = checkout
        .as_ref()
        .map(|item| cache_key(tenant, &item.root.to_string_lossy(), item.origin.as_deref()));
    if !force
        && let Some(key) = &key
        && let Ok(cached) = api_cache::read(api.server(), &api.token, key)
        && let Some(canonical) = cached["slug"].as_str()
    {
        return Ok(canonical.to_owned());
    }
    if api.offline() && request_outbox::has_project_init(api.server(), &base, &slug)? {
        return Ok(slug);
    }
    let mut body = json!({"slug": slug, "name": project::name(None)?,
        "remote_url": checkout.as_ref().and_then(|item| item.origin.clone()),
        "provides": {"auto": project_provides::detect(), "manual": []}});
    if let Some(checkout) = &checkout {
        body["host"] = json!(attribution::host());
        body["path"] = json!(checkout.root);
        body["kind"] = json!(checkout.kind);
        body["branch"] = json!(checkout.branch);
        body["main_path"] = json!(checkout.main_path());
    }
    let created = match api.post(&base, &body) {
        Ok(value) => value["project"]["slug"]
            .as_str()
            .unwrap_or(&slug)
            .to_owned(),
        Err(error) if error.downcast_ref::<QueuedWrite>().is_some() => slug,
        Err(error) => return Err(error),
    };
    if let Some(key) = key {
        api_cache::write(api.server(), &api.token, &key, &json!({"slug": created}))?;
    }
    Ok(created)
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
        let key = cache_key(parts[3], path, body["remote_url"].as_str());
        api_cache::write(api.server(), &api.token, &key, &json!({"slug": new}))?;
    }
    Ok(Some((
        format!("{}/{}", request.path, old),
        format!("{}/{}", request.path, new),
    )))
}

fn cache_key(tenant: &str, path: &str, remote: Option<&str>) -> String {
    format!("/local-project/{tenant}/{path}/{}", remote.unwrap_or(""))
}

pub fn replace_slug(path: &str, slug: &str) -> String {
    let mut parts = path.split('/').collect::<Vec<_>>();
    if parts.len() > 6 {
        parts[6] = slug;
    }
    parts.join("/")
}
