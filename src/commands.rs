use crate::api::Api;
use crate::output;
use crate::project;
use crate::refs;
use crate::{attribution, git, staleness};
use anyhow::Result;
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

pub fn add(
    api: &Api,
    tenant: &str,
    project: Option<&str>,
    kind: &str,
    title: &str,
    body: &str,
    topics: Vec<String>,
    raw_refs: Vec<String>,
    agent: Option<String>,
    force: bool,
    global_scope: bool,
    json_mode: bool,
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
    let response = api.post(
        &format!("{}/entries", path(tenant, project)?),
        &json!({"kind": kind, "title": title, "body": body, "topics": topics, "refs": refs,
            "agent": attribution::agent(agent.as_deref()), "host": attribution::host(),
            "force": force, "scope": if global_scope { "global" } else { "project" }}),
    )?;
    let id = response["entry"]["id"]
        .as_str()
        .unwrap_or("")
        .replace('-', "");
    output::emit(
        &response,
        &format!("Recorded {} ({kind}): {title}", &id[..id.len().min(8)]),
        json_mode,
    )
}
