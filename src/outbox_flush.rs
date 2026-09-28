use crate::api::Api;
use crate::{hook_project, outbox};
use anyhow::{Result, bail};
use fs2::FileExt;
use serde_json::json;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::path::Path;

pub fn run(api: &Api, tenant: &str) -> Result<usize> {
    let lock = OpenOptions::new()
        .write(true)
        .create(true)
        .open(outbox::directory()?.join(".flush.lock"))?;
    if lock.try_lock_exclusive().is_err() {
        return Ok(0);
    }
    let mut sent = 0;
    loop {
        let batch = outbox::entries()?.into_iter().take(100).collect::<Vec<_>>();
        if batch.is_empty() {
            break;
        }
        let mut identities = HashMap::<String, Option<String>>::new();
        let mut events = Vec::new();
        for (_, original) in &batch {
            let mut event = original.clone();
            if event["project_explicit"] != true
                && let Some(path) = event["checkout_path"].as_str()
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
        for (path, event) in batch {
            if !acknowledged.iter().any(|id| id == &event["id"]) {
                bail!("server did not acknowledge event {}", event["id"]);
            }
            fs::remove_file(path)?;
            sent += 1;
        }
    }
    Ok(sent)
}
