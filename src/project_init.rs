use crate::api::Api;
use crate::{attribution, checkout_identity, output, project, project_folder, project_provides};
use anyhow::Result;
use serde_json::json;

/// Creates or attaches the project for a Git checkout or, outside Git, registers the folder.
pub fn run(
    api: &Api,
    tenant: &str,
    explicit: Option<&str>,
    name: Option<&str>,
    json_mode: bool,
) -> Result<()> {
    let checkout = checkout_identity::current();
    let folder = checkout
        .is_none()
        .then(project_folder::current_dir)
        .flatten();
    let slug = match (&folder, explicit) {
        (Some(path), None) => project::folder_slug(path),
        _ => project::slug(explicit)?,
    };
    let mut body = json!({"slug": slug, "name": project::name(name)?,
        "remote_url": checkout.as_ref().and_then(|item| item.origin.clone()),
        "provides": {"auto": project_provides::detect(), "manual": []}});
    if let Some(checkout) = &checkout {
        body["host"] = json!(attribution::host());
        body["path"] = json!(checkout.root);
        body["kind"] = json!(checkout.kind);
        body["branch"] = json!(checkout.branch);
        body["main_path"] = json!(checkout.main_path());
    } else if let Some(path) = &folder {
        body["host"] = json!(attribution::host());
        body["path"] = json!(path);
        body["kind"] = json!("folder");
    }
    let result = api.post(&format!("/api/v1/tenants/{tenant}/projects"), &body)?;
    let recorded = result["project"]["slug"].as_str().unwrap_or(&slug);
    let text = if folder.is_some() {
        format!("Initialized project {recorded} for this folder.")
    } else {
        format!("Initialized project {recorded}.")
    };
    output::emit(&result, &text, json_mode)
}
