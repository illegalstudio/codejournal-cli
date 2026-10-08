use crate::{
    api::Api,
    request_outbox::{self, PendingRequest},
};
use anyhow::{Result, bail};
use serde_json::Value;

fn ids(path: &str, body: Option<&Value>) -> Vec<(String, String)> {
    let parts = path.split('/').collect::<Vec<_>>();
    let mut found = parts
        .windows(2)
        .filter(|pair| ["entries", "plans", "docs", "tasks", "logs"].contains(&pair[0]))
        .map(|pair| {
            (
                pair[1].split('?').next().unwrap_or("").to_owned(),
                pair[0].to_owned(),
            )
        })
        .collect::<Vec<_>>();
    if let Some(body) = body {
        for (key, kind) in [
            ("by", "entries"),
            ("plan_id", "plans"),
            ("plan", "plans"),
            ("from_entry", "entries"),
            ("source_entry_id", "entries"),
        ] {
            if let Some(id) = body[key].as_str() {
                found.push((id.to_owned(), kind.to_owned()));
            }
        }
    }
    found
}

pub fn reject_pending(api: &Api, path: &str, body: Option<&Value>) -> Result<()> {
    let tenant = path.split('/').nth(4).unwrap_or("");
    let references = ids(path, body);
    if references.is_empty() {
        return Ok(());
    }
    for (_, request) in request_outbox::entries()? {
        if request.server != api.server() || request.path.split('/').nth(4) != Some(tenant) {
            continue;
        }
        if references.iter().any(|(id, _)| {
            let compact = id.replace('-', "").to_ascii_lowercase();
            compact.len() >= 8 && request.id.replace('-', "").starts_with(&compact)
        }) {
            bail!(
                "{} is a pending request_id, not a resource ID. This operation was not sent or queued. Run cj sync, then cj outbox receipt {} to obtain the resource_id",
                request.id,
                request.id
            );
        }
    }
    Ok(())
}

pub fn receipt(api: &Api, tenant: &str, id: &str) -> Result<Value> {
    if uuid::Uuid::parse_str(id).is_err() {
        bail!("receipt requires the full request_id UUID");
    }
    api.get(&format!("/api/v1/tenants/{tenant}/requests/{id}"))
}

pub fn resolve(api: &Api, request: &mut PendingRequest) -> Result<()> {
    let tenant = request.path.split('/').nth(4).unwrap_or("").to_owned();
    for (old, kind) in ids(&request.path, request.body.as_ref()) {
        if uuid::Uuid::parse_str(&old).is_err() {
            continue;
        }
        let result = match api.get_fresh(&format!("/api/v1/tenants/{tenant}/requests/{old}")) {
            Ok(value) => value,
            Err(error) if error.to_string().contains("404") => continue,
            Err(error) => return Err(error),
        };
        if result["resource_type"] != kind {
            bail!("request receipt {old} is not a {kind} resource");
        }
        let new = result["resource_id"]
            .as_str()
            .filter(|value| uuid::Uuid::parse_str(value).is_ok())
            .ok_or_else(|| anyhow::anyhow!("invalid resource ID in request receipt {old}"))?;
        request.path = request
            .path
            .split('/')
            .map(|part| if part == old { new } else { part })
            .collect::<Vec<_>>()
            .join("/");
        if let Some(body) = &mut request.body {
            for key in ["by", "plan_id", "plan", "from_entry", "source_entry_id"] {
                if body[key] == old {
                    body[key] = Value::String(new.to_owned());
                }
            }
        }
    }
    Ok(())
}
