use crate::api::Api;
use crate::{hook_project, outbox};
use anyhow::{Result, bail};
use fs2::FileExt;
use serde_json::json;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::path::Path;

#[derive(Default)]
pub struct Delivery {
    pub sent: usize,
    pub expired: usize,
    /// Unbound events removed after their 48-hour recovery window.
    pub unbound_removed: usize,
}

pub fn run(api: &Api, tenant: &str) -> Result<Delivery> {
    let lock = OpenOptions::new()
        .write(true)
        .create(true)
        .open(outbox::directory()?.join(".flush.lock"))?;
    if lock.try_lock_exclusive().is_err() {
        return Ok(Delivery::default());
    }
    let mut delivery = Delivery {
        unbound_removed: outbox::expiry::sweep()?,
        ..Delivery::default()
    };
    loop {
        let origin = outbox::fingerprint(&api.server, tenant, &api.token);
        let batch = outbox::entries()?
            .into_iter()
            .filter(|(_, event)| event["origin"].as_str() == Some(origin.as_str()))
            .take(100)
            .collect::<Vec<_>>();
        if batch.is_empty() {
            break;
        }
        let mut identities = HashMap::<String, Option<String>>::new();
        let mut events = Vec::new();
        for (_, original) in &batch {
            let mut event = original.clone();
            crate::hook::delegation_timestamps::normalize(&mut event)?;
            if let Some(fields) = event.as_object_mut() {
                fields.remove("origin");
                fields.remove("origin_unavailable_reason");
            }
            if event["project_explicit"] != true
                && let Some(path) = event["checkout_path"]
                    .as_str()
                    .or_else(|| event["cwd"].as_str())
            {
                let slug = if let Some(cached) = identities.get(path) {
                    cached.clone()
                } else {
                    let resolved = hook_project::resolve(api, tenant, Path::new(path))?;
                    identities.insert(path.to_owned(), resolved.clone());
                    resolved
                };
                event["project"] = json!(slug);
                event["project_resolved"] = json!(slug.is_some());
            }
            events.push(event);
        }
        let result = api.post_noqueue(
            &format!("/api/v1/tenants/{tenant}/client-events"),
            &json!({"events": events}),
        )?;
        let acknowledged = result["acknowledged"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("server did not acknowledge the batch"))?;
        let empty = Vec::new();
        let expired = result["expired"].as_array().unwrap_or(&empty);
        for (path, event) in batch {
            if expired.contains(&event["id"]) && !acknowledged.contains(&event["id"]) {
                let id = event["id"].as_str().unwrap_or("unknown");
                eprintln!("Hook event {id} expired before delivery");
                fs::remove_file(path)?;
                delivery.expired += 1;
                continue;
            }
            if !acknowledged.iter().any(|id| id == &event["id"]) {
                bail!("server did not acknowledge event {}", event["id"]);
            }
            fs::remove_file(path)?;
            delivery.sent += 1;
        }
    }
    Ok(delivery)
}
