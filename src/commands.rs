use crate::api::Api;
use crate::cli::ProjectAction;
use crate::output;
use crate::project;
use crate::refs;
use crate::{git, staleness};
use anyhow::{Result, bail};
use serde_json::json;

fn root(tenant: &str) -> String {
    format!("/api/v1/tenants/{tenant}")
}

pub(crate) fn path(tenant: &str, explicit_project: Option<&str>) -> Result<String> {
    Ok(format!(
        "{}/projects/{}",
        root(tenant),
        project::slug(explicit_project)?
    ))
}

pub fn brief(api: &Api, tenant: &str, project: Option<&str>) -> Result<()> {
    let project_path = path(tenant, project)?;
    let mut result = api.get(&format!("{project_path}/brief"))?;
    if project.is_none() {
        staleness::enrich(api, &project_path, &mut result)?;
    }
    output::json(&result)
}

pub fn search(api: &Api, tenant: &str, project: Option<&str>, query: &str) -> Result<()> {
    let query = reqwest::Url::parse_with_params("http://local/", &[("q", query)])?;
    let project_path = path(tenant, project)?;
    let mut result = api.get(&format!(
        "{}/search?{}",
        project_path,
        query.query().unwrap_or("")
    ))?;
    if project.is_none() {
        staleness::enrich(api, &project_path, &mut result)?;
    }
    output::json(&result)
}

pub fn garden(api: &Api, tenant: &str, project: Option<&str>, dry_run: bool) -> Result<()> {
    if !dry_run {
        bail!("only garden --dry-run is implemented");
    }
    let project_path = path(tenant, project)?;
    let mut entries = Vec::new();
    let mut page = 1_u64;
    loop {
        let mut result = api.get(&format!("{project_path}/garden?page={page}"))?;
        if project.is_none() {
            staleness::enrich(api, &project_path, &mut result)?;
        }
        entries.extend(result["entries"].as_array().cloned().unwrap_or_default());
        let Some(next) = result["next_page"].as_u64() else {
            break;
        };
        if next <= page {
            bail!("invalid garden pagination");
        }
        page = next;
    }
    let stale: Vec<_> = entries
        .iter()
        .filter(|entry| {
            entry["staleness"]["missing"]
                .as_array()
                .is_some_and(|items| !items.is_empty())
        })
        .cloned()
        .collect();
    let elsewhere: Vec<_> = entries
        .iter()
        .filter(|entry| {
            entry["staleness"]["elsewhere"]
                .as_array()
                .is_some_and(|items| !items.is_empty())
        })
        .cloned()
        .collect();
    output::json(&json!({"dry_run": dry_run, "stale_entries": stale,
        "other_branch_entries": elsewhere}))
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
    let mut raw_refs = raw_refs;
    if project.is_none() {
        if let Some(branch) = git::output(&["symbolic-ref", "--quiet", "--short", "HEAD"]) {
            if staleness::default_branch().as_deref() != Some(branch.as_str()) {
                raw_refs.push(format!("branch:{branch}"));
            }
        }
    }
    let refs = refs::parse_all(&raw_refs)?;
    output::json(&api.post(
        &format!("{}/entries", path(tenant, project)?),
        &json!({"kind": kind, "title": title, "body": body, "topics": topics, "refs": refs}),
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
