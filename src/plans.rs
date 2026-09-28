use crate::api::Api;
use crate::plan_args::PlanAction;
use crate::{input, output, project};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

pub fn run(
    api: &Api,
    tenant: &str,
    project_name: Option<&str>,
    kind: &str,
    action: PlanAction,
) -> Result<()> {
    let endpoint = format!(
        "/api/v1/tenants/{tenant}/projects/{}/{kind}",
        project::slug(project_name)?
    );
    match action {
        PlanAction::List => output::json(&api.get(&endpoint)?),
        PlanAction::Create {
            title,
            body,
            body_file,
            status,
            not_before,
        } => {
            let body = input::body(body, body_file)?;
            output::json(&api.post(
                &endpoint,
                &json!({"title": title, "body": body, "status": status, "not_before": not_before}),
            )?)
        }
        PlanAction::Show { id, body } => {
            let id = resolve_id(api, &endpoint, kind, &id)?;
            let result = api.get(&format!("{endpoint}/{id}"))?;
            if body {
                println!(
                    "{}",
                    result[record_key(kind)]["body"]
                        .as_str()
                        .context("missing body")?
                );
                Ok(())
            } else {
                output::json(&result)
            }
        }
        PlanAction::Update {
            id,
            title,
            body,
            body_file,
            note,
            based_on,
            status,
            not_before,
        } => {
            if title.is_none()
                && body.is_none()
                && body_file.is_none()
                && note.is_none()
                && status.is_none()
                && not_before.is_none()
            {
                bail!("nothing to update");
            }
            let body = if body.is_some() || body_file.is_some() {
                Some(input::body(body, body_file)?)
            } else {
                None
            };
            let id = resolve_id(api, &endpoint, kind, &id)?;
            let mut payload = json!({"based_on": based_on});
            insert_if_some(&mut payload, "title", title);
            insert_if_some(&mut payload, "body", body);
            insert_if_some(&mut payload, "note", note);
            insert_if_some(&mut payload, "status", status);
            insert_if_some(&mut payload, "not_before", not_before);
            output::json(&api.patch(&format!("{endpoint}/{id}"), &payload)?)
        }
        PlanAction::Schedule { id, date } => {
            let id = resolve_id(api, &endpoint, kind, &id)?;
            output::json(&api.patch(&format!("{endpoint}/{id}"), &json!({"not_before": date}))?)
        }
    }
}

fn record_key(kind: &str) -> &str {
    if kind == "docs" { "doc" } else { "plan" }
}

fn resolve_id(api: &Api, endpoint: &str, kind: &str, prefix: &str) -> Result<String> {
    let listing = api.get(endpoint)?;
    let rows = listing[kind].as_array().context("invalid plan listing")?;
    let matches: Vec<&str> = rows
        .iter()
        .filter_map(|row| row["id"].as_str())
        .filter(|id| id.replace('-', "").starts_with(&prefix.replace('-', "")))
        .collect();
    match matches.as_slice() {
        [id] => Ok((*id).to_owned()),
        [] => bail!("no {kind} record matches {prefix}"),
        _ => bail!("{kind} prefix {prefix} is ambiguous"),
    }
}

fn insert_if_some(payload: &mut Value, key: &str, value: Option<String>) {
    if let Some(value) = value {
        payload[key] = Value::String(value);
    }
}
