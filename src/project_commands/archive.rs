use crate::{api::Api, api_cache, output, project_bootstrap};
use anyhow::{Result, bail};
use serde_json::{Value, json};

pub fn run(api: &Api, tenant: &str, slug: &str, archived: bool, json_mode: bool) -> Result<()> {
    if api.offline() {
        bail!("archiving and restoring a project require an online connection");
    }
    let base =
        project_bootstrap::read_path(api, &format!("/api/v1/tenants/{tenant}/projects/{slug}"))?;
    let result = api.post_noqueue(&format!("{base}/archive"), &json!({"archived": archived}))?;
    remember(
        api,
        &base,
        &json!({"archived": archived, "project": result["project"]}),
    )?;
    output::emit(
        &result,
        &format!(
            "{} project {}.",
            if archived { "Archived" } else { "Restored" },
            result["project"]["slug"].as_str().unwrap_or(slug)
        ),
        json_mode,
    )
}

pub fn remember(api: &Api, base: &str, brief: &Value) -> Result<()> {
    api_cache::write(
        api.server(),
        &api.token,
        &format!("{base}/archive-state"),
        &json!({"archived": brief["archived"] == true, "project": {"id": brief["project"]["id"], "slug": brief["project"]["slug"],
            "name": brief["project"]["name"], "archived_at": brief["project"]["archived_at"]}}),
    )
}

pub fn cached(api: &Api, base: &str) -> Option<Value> {
    let known = api_cache::read(api.server(), &api.token, &format!("{base}/archive-state")).ok()?;
    (known["archived"] == true).then(|| {
        json!({"archived": true, "project": known["project"],
        "message": "This project is archived and read-only. Restore it explicitly before writing."})
    })
}

pub fn message(brief: &Value) -> String {
    let slug = brief["project"]["slug"].as_str().unwrap_or("SLUG");
    format!(
        "Project: {slug}\nArchived: read-only. History is available through explicit reads.\nRestore only when the user asks: cj project restore --project {slug}"
    )
}

pub fn current() -> Option<Value> {
    let config = crate::config::Config::load().ok()?;
    let token = config.token().ok()?;
    let mut api = Api::new(&config.server, &token).ok()?;
    api.set_offline(true);
    let preferred = match crate::project_folder::scope(&api, &config.tenant) {
        crate::project_folder::Scope::Named(slug) => Some(slug),
        crate::project_folder::Scope::Checkout => std::env::var("CJ_PROJECT").ok(),
        _ => return None,
    };
    api.set_auto_project(preferred.is_none());
    let slug = crate::project::slug(preferred.as_deref()).ok()?;
    let path = format!("/api/v1/tenants/{}/projects/{slug}", config.tenant);
    let base = project_bootstrap::read_path(&api, &path).ok()?;
    cached(&api, &base)
}

pub fn validate_cached(api: &Api, base: &str, brief: Value) -> Result<Value> {
    let known = api_cache::read(api.server(), &api.token, &format!("{base}/archive-state"));
    if brief["archived"] == true && known.is_ok_and(|state| state["archived"] == false) {
        bail!("This project was restored. Run cj brief online to refresh its context.");
    }
    Ok(brief)
}
