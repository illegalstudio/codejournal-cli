use super::{authorization, launch, process};
use crate::{api::Api, attribution, outbox, watch_state};
use anyhow::{Context, Result};
use serde_json::json;

pub fn resume(api: &Api, tenant: &str) -> Result<usize> {
    let mut started = 0;
    let scope = outbox::fingerprint(api.server(), tenant, &api.token);
    for (id, authorized) in authorization::pending(api, tenant)? {
        if watch_state::read_pid(&id, &scope).is_ok_and(|pid| process::owns(pid, &id)) {
            continue;
        }
        let result = api.get_fresh(&format!("{}/{id}", authorized.endpoint))?;
        let watch = &result["watch"];
        anyhow::ensure!(
            watch["id"] == id,
            "invalid fresh watch response; unused authorization retained"
        );
        if matches!(
            watch["status"].as_str(),
            Some("finished" | "timed_out" | "cancelled")
        ) {
            authorization::remove(&id);
            continue;
        }
        anyhow::ensure!(
            matches!(
                watch["status"].as_str(),
                Some("starting" | "running" | "lost")
            ),
            "invalid fresh watch status; unused authorization retained"
        );
        authorization::verify(&authorized, watch, &id)?;
        let project = authorized.endpoint.split('/').nth(6).unwrap_or("");
        launch::run(api, tenant, project, &id)?;
        started += 1;
    }
    Ok(started)
}

pub fn reconcile(api: &Api, tenant: &str) -> Result<usize> {
    let fresh = super::lease::client(api)?;
    let query = reqwest::Url::parse_with_params("http://local/", [("host", attribution::host())])?;
    let listing = fresh.get_fresh(&format!(
        "/api/v1/tenants/{tenant}/watches?{}",
        query.query().unwrap_or("")
    ))?;
    let scope = outbox::fingerprint(api.server(), tenant, &api.token);
    let mut lost = 0;
    let watches = listing["watches"]
        .as_array()
        .context("invalid active watch response")?;
    for watch in watches {
        let Some(id) = watch["id"].as_str() else {
            continue;
        };
        if watch["host"] == attribution::host()
            && matches!(watch["status"].as_str(), Some("starting" | "running"))
            && watch["lease_expires_at"].is_null()
            && authorization::load(
                api,
                tenant,
                &format!(
                    "{}/watches",
                    super::endpoint(tenant, watch["project_slug"].as_str().unwrap_or(""))
                ),
                id,
            )
            .is_err()
            && !watch_state::read_pid(id, &scope).is_ok_and(|pid| process::owns(pid, id))
            && !running_without_record(id)
        {
            let Some(project) = watch["project_slug"].as_str() else {
                continue;
            };
            api.patch(&format!("{}/watches/{id}", super::endpoint(tenant, project)),
                &json!({"status": "lost", "host": attribution::host(),
                    "tail": "No local runner could be found. The command was not restarted; its outcome is unknown."}))?;
            lost += 1;
        }
    }
    Ok(lost)
}

fn running_without_record(id: &str) -> bool {
    let Ok(output) = std::process::Command::new("ps")
        .args(["-axo", "pid="])
        .output()
    else {
        return true;
    };
    if !output.status.success() {
        return true;
    }
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .filter_map(|pid| pid.parse().ok())
        .any(|pid| process::owns(pid, id))
}
