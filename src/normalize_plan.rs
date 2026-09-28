use crate::{api::Api, attribution, checkout_identity};
use anyhow::Result;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub fn build(api: &Api, tenant: &str) -> Result<Value> {
    let result = api.get(&format!("/api/v1/tenants/{tenant}/checkouts"))?;
    let paths = result["checkouts"].as_array().cloned().unwrap_or_default();
    let host = attribution::host();
    let other_hosts = paths.iter().filter(|row| row["host"] != host).count();
    let local = paths
        .iter()
        .filter(|row| row["host"] == host)
        .collect::<Vec<_>>();
    let mut main_counts = HashMap::<String, usize>::new();
    for row in &local {
        if row["kind"] == "main" {
            *main_counts
                .entry(value(&row["project_id"]).to_owned())
                .or_default() += 1;
        }
    }
    let projects = api.get(&format!("/api/v1/tenants/{tenant}/projects"))?;
    let candidates = projects["projects"].as_array().cloned().unwrap_or_default();
    let mut merges = Vec::new();
    let mut merged = HashSet::new();
    let mut removals = Vec::new();
    let mut updates = Vec::new();
    let mut mains = Vec::new();
    let mut kept = Vec::new();
    for row in local {
        let path = Path::new(value(&row["path"]));
        let info = checkout_identity::at(path, 0);
        let root = info.as_ref().map(|checkout| checkout.root.clone());
        let canonical = path.canonicalize().ok();
        if !path.is_dir() || root != canonical {
            let id = value(&row["project_id"]);
            let anchor = row["remote_url"].is_null()
                && row["kind"] == "main"
                && main_counts.get(id).copied().unwrap_or(0) <= 1;
            if anchor {
                kept.push(json!({"slug": row["project_slug"], "path": row["path"],
                "reason": "only path of a project without a remote"}));
            } else {
                removals.push(json!({"slug": row["project_slug"], "path": row["path"],
                    "reason": if path.is_dir() { "not a Git checkout root" } else { "path no longer exists" }}));
                if row["kind"] == "main" {
                    *main_counts.entry(id.to_owned()).or_default() -= 1;
                }
            }
            continue;
        }
        let info = info.unwrap();
        let remote = info.origin.clone();
        let target = remote
            .as_ref()
            .and_then(|remote| candidates.iter().find(|item| item["remote_url"] == *remote));
        let target_slug = target
            .map(|item| value(&item["slug"]))
            .or_else(|| {
                remote
                    .is_none()
                    .then(|| {
                        paths.iter().find(|item| {
                            item["host"] == host
                                && item["kind"] == "main"
                                && item["path"] == info.anchor.display().to_string()
                        })
                    })
                    .flatten()
                    .map(|item| value(&item["project_slug"]))
            })
            .unwrap_or_else(|| value(&row["project_slug"]));
        if target_slug != value(&row["project_slug"])
            && merged.insert(value(&row["project_slug"]).to_owned())
        {
            merges.push(json!({"from": row["project_slug"], "into": target_slug}));
        }
        let main_path = info.main_path().map(|path| path.display().to_string());
        let kind = info.kind;
        let branch = if kind == "main" { None } else { info.branch };
        if row["kind"] != kind
            || row["branch"].as_str() != branch.as_deref()
            || row["main_path"].as_str() != main_path.as_deref()
        {
            updates.push(
                json!({"slug": target_slug, "path": row["path"], "kind": row["kind"],
                "new_kind": kind, "new_branch": branch, "new_main_path": main_path}),
            );
        }
        if let Some(main) = main_path {
            if !paths
                .iter()
                .any(|item| item["host"] == host && item["path"] == main)
                && !mains.iter().any(|item: &Value| item["path"] == main)
            {
                mains.push(json!({"slug": target_slug, "path": main}));
            }
        }
        kept.push(json!({"slug": target_slug, "path": row["path"], "reason": "valid checkout"}));
    }
    let changes = merges.len() + removals.len() + updates.len() + mains.len();
    Ok(json!({"host": host, "changes": changes, "merges": merges,
        "removals": removals, "updates": updates, "add_main": mains,
        "kept": kept, "other_hosts": other_hosts}))
}

fn value(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
