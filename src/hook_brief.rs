use crate::project_folder::{self, Scope};
use crate::{
    api::Api, brief_focus, brief_format, config::Config, outbox, project, project_bootstrap,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn online(source: &str, agent: &str, session: &str) -> Option<String> {
    let config = Config::load().ok()?;
    let token = config.token().ok()?;
    let mut api = Api::with_timeout(&config.server, &token, Duration::from_secs(3)).ok()?;
    let preferred = match project_folder::scope(&api, &config.tenant) {
        Scope::Named(slug) => Some(slug),
        Scope::Checkout => None,
        Scope::Outside => return Some(project_folder::notice()),
        Scope::Unknown => return None,
    };
    api.set_auto_project(preferred.is_none());
    let slug = project::slug(preferred.as_deref()).ok()?;
    let query = reqwest::Url::parse_with_params(
        "http://local/",
        [
            ("limit", "10"),
            ("pinned_limit", "15"),
            ("log_limit", "5"),
            ("agent", agent),
            ("session", session),
        ],
    )
    .ok()?;
    let path = format!(
        "/api/v1/tenants/{}/projects/{slug}/brief?{}",
        config.tenant,
        query.query()?
    );
    let base = format!("/api/v1/tenants/{}/projects/{slug}", config.tenant);
    let result = brief_focus::load(
        &api,
        &base,
        &path,
        json!({
            "limit": 10, "pinned_limit": 15, "log_limit": 5, "agent": agent, "session": session,
        }),
    )
    .or_else(|error| {
        if preferred.is_some() || !error.to_string().contains("404") {
            return Err(error);
        }
        project_bootstrap::ensure(&api, &config.tenant, false)?;
        brief_focus::load(
            &api,
            &base,
            &path,
            json!({
                "limit": 10, "pinned_limit": 15, "log_limit": 5,
                "agent": agent, "session": session,
            }),
        )
    })
    .ok()?;
    let body = brief_format::render(&result, source == "compact", 10, 15, 5);
    Some(format!(
        "Code Journal brief for this repository (injected by the cj session hook; you do not need to run `cj brief` again unless it says so):\n\n{body}"
    ))
}

pub fn cache_path(cwd: &Path) -> Option<PathBuf> {
    let parent = outbox::directory().ok()?.parent()?.to_path_buf();
    let digest = Sha256::digest(cwd.to_string_lossy().as_bytes());
    Some(parent.join("briefs").join(format!("{digest:x}.txt")))
}
