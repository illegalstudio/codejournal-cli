use crate::api::Api;
use crate::cli::ProjectAction;
use crate::output;
use crate::project;
use crate::refs;
use anyhow::Result;
use serde_json::json;

fn root(tenant: &str) -> String {
    format!("/api/v1/tenants/{tenant}")
}

fn path(tenant: &str, explicit_project: Option<&str>) -> Result<String> {
    Ok(format!(
        "{}/projects/{}",
        root(tenant),
        project::slug(explicit_project)?
    ))
}

pub fn brief(api: &Api, tenant: &str, project: Option<&str>) -> Result<()> {
    output::json(&api.get(&format!("{}/brief", path(tenant, project)?))?)
}

pub fn search(api: &Api, tenant: &str, project: Option<&str>, query: &str) -> Result<()> {
    let query = reqwest::Url::parse_with_params("http://local/", &[("q", query)])?;
    output::json(&api.get(&format!(
        "{}/search?{}",
        path(tenant, project)?,
        query.query().unwrap_or("")
    ))?)
}

pub fn add(
    api: &Api,
    tenant: &str,
    project: Option<&str>,
    kind: &str,
    title: &str,
    body: &str,
    topics: Vec<String>,
    raw_refs: Vec<String>,
) -> Result<()> {
    let refs = refs::parse_all(&raw_refs)?;
    output::json(&api.post(
        &format!("{}/entries", path(tenant, project)?),
        &json!({"kind": kind, "title": title, "body": body, "topics": topics, "refs": refs}),
    )?)
}

pub fn log(
    api: &Api,
    tenant: &str,
    project: Option<&str>,
    title: &str,
    body: &str,
    raw_refs: Vec<String>,
) -> Result<()> {
    let refs = refs::parse_all(&raw_refs)?;
    output::json(&api.post(
        &format!("{}/logs", path(tenant, project)?),
        &json!({"title": title, "body": body, "refs": refs}),
    )?)
}

pub fn project(
    api: &Api,
    tenant: &str,
    explicit: Option<&str>,
    action: ProjectAction,
) -> Result<()> {
    let endpoint = format!("{}/projects", root(tenant));
    match action {
        ProjectAction::List => output::json(&api.get(&endpoint)?),
        ProjectAction::Init { name } => output::json(&api.post(
            &endpoint,
            &json!({"slug": project::slug(explicit)?, "name": project::name(name.as_deref())?}),
        )?),
    }
}
