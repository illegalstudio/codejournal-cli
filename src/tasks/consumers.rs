use crate::{api::Api, output, project_bootstrap};
use anyhow::Result;
use serde_json::json;

pub fn change(
    api: &Api,
    tenant: &str,
    project: Option<&str>,
    id: &str,
    follow: bool,
    json_mode: bool,
) -> Result<()> {
    let slug = project_bootstrap::resolved_slug(api, tenant, project)?;
    let path = format!("/api/v1/tenants/{tenant}/tasks/{id}/consumers/{slug}");
    let value = if follow {
        api.put(&path, &json!({}))?
    } else {
        api.delete(&path)?
    };
    let message = if follow {
        format!("Project {slug} is following task {id}.")
    } else {
        format!("Project {slug} stopped following task {id}.")
    };
    output::emit(&value, &message, json_mode)
}
