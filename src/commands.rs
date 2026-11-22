use crate::api::Api;
use crate::output;
use crate::project;
use crate::refs;
use crate::{attribution, git, staleness};
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

/// The kinds the server keeps as temporary observations; checked here so no request is sent for the others.
const OBSERVATIONS: [&str; 3] = ["discovery", "gotcha", "howto"];

fn temporary_kind(kind: &str, temporary: bool) -> Result<()> {
    if temporary && !OBSERVATIONS.contains(&kind) {
        bail!(
            "--temporary applies to project observations ({}); a {kind} entry is kept until it is superseded or marked obsolete, so omit --temporary",
            OBSERVATIONS.join(", ")
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
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
    supersedes: Option<String>,
    global_scope: bool,
    temporary: bool,
    durable: bool,
    json_mode: bool,
) -> Result<()> {
    temporary_kind(kind, temporary)?;
    let mut raw_refs = raw_refs;
    if project.is_none() {
        if let Some(branch) = git::output(&["symbolic-ref", "--quiet", "--short", "HEAD"]) {
            if staleness::default_branch().as_deref() != Some(branch.as_str()) {
                raw_refs.push(format!("branch:{branch}"));
            }
        }
    }
    let refs = refs::parse_all(&raw_refs)?;
    let mut payload = json!({"kind": kind, "title": title, "body": body, "topics": topics, "refs": refs,
        "agent": attribution::agent(agent.as_deref()), "host": attribution::host(),
        "force": force, "scope": if global_scope { "global" } else { "project" }});
    if durable {
        payload["durable"] = json!(true);
    }
    if let Some(old) = &supersedes {
        payload["supersedes"] = json!(old);
    }
    let mut response = api.post(
        &format!(
            "{}/entries{}",
            path(tenant, project)?,
            if temporary { "/temporary" } else { "" }
        ),
        &payload,
    )?;
    response["id"] = response["entry"]["id"].clone();
    response["resource_id"] = response["entry"]["id"].clone();
    response["queued"] = json!(false);
    let id = response["entry"]["id"]
        .as_str()
        .unwrap_or("")
        .replace('-', "");
    let short_id = &id[..id.len().min(8)];
    let mut text = format!("Recorded {short_id} ({kind}): {title}");
    if let Some(old) = &supersedes {
        // An older server ignores the field and leaves the old entry active.
        match response["superseded"].as_str() {
            Some(done) => text.push_str(&format!(
                "\nSuperseded {}.",
                &done.replace('-', "")[..8.min(done.len())]
            )),
            None => text.push_str(&format!(
                "\nwarning: this server did not supersede {old}; run `cj supersede {old} --by {short_id}`"
            )),
        }
    }
    if let Some(expiry) = response["entry"]["expires_at"].as_str() {
        text.push_str(&format!(
            "\nExpires {expiry}. Promote with `cj entry promote {short_id}`."
        ));
    }
    if let Some(mentions) = response["mentions"].as_array() {
        for mention in mentions {
            let slug = mention["slug"].as_str().unwrap_or("");
            let names = mention["names"]
                .as_array()
                .into_iter()
                .flatten()
                .take(2)
                .filter_map(|name| name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            text.push_str(&format!(
                "\nhint: this entry mentions {names}, the user's project {slug}. If it is a problem or missing feature of that project, forward it: `cj task add --to {slug} --from-entry {short_id} --title ...`"
            ));
        }
    }
    output::emit(&response, &text, json_mode)
}
