use crate::api::Api;
use crate::{attribution, commands, input, output, refs};
use anyhow::{Result, bail};
use serde_json::{Value, json};

fn noun(kind: &str) -> &str {
    if kind == "docs" { "doc" } else { "plan" }
}
fn path(tenant: &str, kind: &str, id: &str) -> String {
    format!("/api/v1/tenants/{tenant}/{kind}/{id}")
}
fn short(id: &str) -> String {
    id.replace('-', "").chars().take(8).collect()
}

#[allow(clippy::too_many_arguments)]
pub fn create(
    api: &Api,
    tenant: &str,
    project: Option<&str>,
    kind: &str,
    global: bool,
    title: String,
    body: Option<String>,
    body_file: Option<String>,
    status: Option<String>,
    not_before: Option<String>,
    raw_refs: Vec<String>,
    agent: Option<String>,
    json_mode: bool,
) -> Result<()> {
    let body = input::body(body, body_file)?;
    crate::plan_validation::body(&body)?;
    let status =
        status.unwrap_or_else(|| if kind == "docs" { "current" } else { "active" }.to_owned());
    let base = if global {
        format!("/api/v1/tenants/{tenant}")
    } else {
        commands::path(tenant, project)?
    };
    let result = api.post(
        &format!("{base}/{kind}"),
        &json!({
            "title": title, "body": body, "status": status, "not_before": not_before,
            "refs": refs::parse_all(&raw_refs)?, "agent": attribution::agent(agent.as_deref()),
            "host": attribution::host(),
        }),
    )?;
    let item = &result[noun(kind)];
    output::emit(
        &json!({noun(kind): item, "queued": false, "warnings": []}),
        &format!(
            "Created {} {} ({}): {}",
            noun(kind),
            short(item["id"].as_str().unwrap_or("")),
            item["status"].as_str().unwrap_or(""),
            item["title"].as_str().unwrap_or("")
        ),
        json_mode,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn update(
    api: &Api,
    tenant: &str,
    kind: &str,
    id: &str,
    title: Option<String>,
    body: Option<String>,
    body_file: Option<String>,
    raw_refs: Vec<String>,
    note: Option<String>,
    based_on: Option<u32>,
    agent: Option<String>,
    json_mode: bool,
) -> Result<()> {
    let markdown = input::optional_body(body, body_file)?;
    crate::plan_validation::body(&markdown)?;
    if title.is_none()
        && markdown.is_empty()
        && raw_refs.is_empty()
        && note.as_deref().is_none_or(str::is_empty)
    {
        bail!("nothing to update; pass Markdown, --title, --ref, or --note");
    }
    let mut payload = json!({"note": note, "based_on": based_on,
        "agent": attribution::agent(agent.as_deref()), "host": attribution::host()});
    if let Some(title) = title {
        payload["title"] = Value::String(title);
    }
    if !markdown.is_empty() {
        payload["body"] = Value::String(markdown);
    }
    if !raw_refs.is_empty() {
        payload["refs"] = json!(refs::parse_all(&raw_refs)?);
    }
    let result = api.patch(&path(tenant, kind, id), &payload)?;
    let item = &result[noun(kind)];
    let mut text = format!(
        "{} {} is now at revision {}.",
        noun(kind),
        short(item["id"].as_str().unwrap_or(id)),
        item["revision"]
    );
    if result["conflict"].as_bool().unwrap_or(false) {
        text.push_str(" Warning: the base revision changed; review history.");
    }
    output::emit(&result, &text, json_mode)
}
