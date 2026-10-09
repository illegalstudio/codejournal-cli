use crate::{api::Api, api_cache, request_outbox};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

mod checklist;
mod content;

pub fn remember(api: &Api, tenant: &str, result: &Value) -> Result<()> {
    for (kind, noun) in [("plans", "plan"), ("docs", "doc")] {
        if let Some(id) = result[noun]["id"].as_str() {
            api_cache::write(
                api.server(),
                &api.token,
                &format!("/api/v1/tenants/{tenant}/{kind}/{id}?history=0"),
                result,
            )?;
        }
    }
    Ok(())
}

pub fn read(api: &Api, path: &str) -> Result<Option<Value>> {
    let parts = path
        .split('?')
        .next()
        .unwrap_or(path)
        .split('/')
        .collect::<Vec<_>>();
    if parts.len() != 7 || !["plans", "docs"].contains(&parts[5]) {
        return Ok(None);
    }
    let (tenant, kind, id) = (parts[4], parts[5], parts[6]);
    let noun = if kind == "docs" { "doc" } else { "plan" };
    let origin = crate::outbox::fingerprint(api.server(), tenant, &api.token);
    let requests = request_outbox::entries()?
        .into_iter()
        .map(|(_, request)| request)
        .filter(|request| {
            request.server == api.server() && request.origin.as_deref() == Some(&origin)
        })
        .collect::<Vec<_>>();
    let base = format!("/api/v1/tenants/{tenant}/{kind}/{id}");
    let creation = requests.iter().find(|request| {
        request.method == "POST"
            && request.path.ends_with(&format!("/{kind}"))
            && request.path.split('/').nth(4) == Some(tenant)
            && request.body.as_ref().is_some_and(|body| body["id"] == id)
    });
    let edits = requests
        .iter()
        .filter(|request| request.method == "PATCH" && request.path == base)
        .collect::<Vec<_>>();
    if creation.is_none() && edits.is_empty() {
        return Ok(None);
    }
    if path.contains("revision=") || path.contains("history=1") || path.contains("history_content=")
    {
        bail!(
            "pending local plans expose current projected content only; synchronize before reading history"
        );
    }
    let mut result = if let Some(creation) = creation {
        let mut item = creation
            .body
            .clone()
            .context("queued plan creation has no body")?;
        item["revision"] = json!(1);
        item["project_slug"] = json!(creation.path.split('/').nth(6));
        item["scope"] = json!(if creation.path.split('/').nth(5) == Some("projects") {
            "project"
        } else {
            "global"
        });
        item["body"] = json!(content::clean(
            item["body"].as_str().unwrap_or(""),
            item["title"].as_str().unwrap_or("")
        )?);
        json!({noun: item})
    } else {
        api_cache::read(api.server(), &api.token, &format!("{base}?history=0"))?
    };
    for edit in edits {
        let payload = edit
            .body
            .as_ref()
            .context("queued plan update has no body")?;
        let item = &mut result[noun];
        let revision = item["revision"]
            .as_u64()
            .context("missing local plan revision")?;
        if payload["action"] == "step" {
            if payload["based_on"].as_u64() != Some(revision) {
                bail!(
                    "queued checklist base differs from cached revision; synchronize and review the conflict"
                );
            }
            item["body"] = json!(checklist::step(
                item["body"].as_str().context("missing local plan body")?,
                payload["step_index"]
                    .as_u64()
                    .context("missing checklist index")?,
                payload["step_done"] == true
            )?);
        }
        for key in ["title", "body", "status", "refs", "not_before"] {
            if payload.get(key).is_some() {
                item[key] = payload[key].clone();
            }
        }
        if payload.get("body").is_some() {
            item["body"] = json!(content::clean(
                item["body"].as_str().unwrap_or(""),
                item["title"].as_str().unwrap_or("")
            )?);
        }
        if payload["action"] == "move" {
            let global = payload["to"] == "@global";
            item["project_slug"] = if global {
                Value::Null
            } else {
                payload["to"].clone()
            };
            item["scope"] = json!(if global { "global" } else { "project" });
        }
        if payload["action"] == "step"
            || payload["action"] == "move"
            || ["title", "body", "status", "refs", "note"]
                .iter()
                .any(|key| payload.get(*key).is_some())
        {
            item["revision"] = json!(revision + 1);
        }
    }
    result["current_revision"] = result[noun]["revision"].clone();
    result["revision_count"] = result[noun]["revision"].clone();
    result["cached"] = json!(true);
    result["local_pending"] = json!(true);
    result["synced"] = json!(false);
    eprintln!("Showing local pending plan content; changes are not synchronized.");
    Ok(Some(crate::api::sanitized(result)))
}
