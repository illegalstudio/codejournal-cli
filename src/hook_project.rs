use crate::{api::Api, api_cache, attribution, checkout_identity, project, project_bootstrap};
use anyhow::Result;
use serde_json::json;
use std::path::Path;

pub fn resolve(api: &Api, tenant: &str, path: &Path) -> Result<Option<String>> {
    let Some(checkout) = checkout_identity::at(path, 0) else {
        return Ok(None);
    };
    let key = project_bootstrap::cache_key(
        tenant,
        &checkout.root.to_string_lossy(),
        checkout.origin.as_deref(),
    );
    if let Ok(value) = api_cache::read(api.server(), &api.token, &key)
        && let Some(slug) = value["slug"].as_str()
    {
        return Ok(Some(slug.to_owned()));
    }
    let body = json!({
        "slug": project::slug_for(&checkout), "name": project::name_for(&checkout),
        "remote_url": checkout.origin, "host": attribution::host(),
        "path": checkout.root, "kind": checkout.kind, "branch": checkout.branch,
        "main_path": checkout.main_path(),
    });
    let response = api.post_noqueue(&format!("/api/v1/tenants/{tenant}/projects"), &body)?;
    let slug = response["project"]["slug"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("project creation returned no slug"))?;
    api_cache::write(api.server(), &api.token, &key, &json!({"slug": slug}))?;
    Ok(Some(slug.to_owned()))
}
