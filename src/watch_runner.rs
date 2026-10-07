use crate::api::Api;
use crate::{
    notification_delivery, watch_state,
    watches::{authorization, execution, lease, process},
};
use anyhow::Result;
use fs2::FileExt;
use serde_json::json;
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom};

pub fn run(api: &Api, tenant: &str, path: &str, id: &str) -> Result<()> {
    let lock = OpenOptions::new()
        .write(true)
        .create(true)
        .open(watch_state::pid_path(id)?.with_extension("execution.lock"))?;
    lock.try_lock_exclusive()?;
    let authorized = authorization::load(api, tenant, path, id)?;
    let scope = crate::outbox::fingerprint(api.server(), tenant, &api.token);
    process::install_signals();
    watch_state::write_pid(id, std::process::id(), &scope)?;
    let result = run_authorized(api, tenant, path, id, authorized);
    watch_state::remove_pid(id);
    result
}

fn run_authorized(
    api: &Api,
    tenant: &str,
    path: &str,
    id: &str,
    authorized: authorization::Authorization,
) -> Result<()> {
    let fresh = lease::client(api)?;
    let watch = fresh.get_fresh(&format!("{path}/{id}"))?["watch"].clone();
    anyhow::ensure!(
        watch["id"] == id,
        "invalid fresh watch response; unused authorization retained"
    );
    if matches!(
        watch["status"].as_str(),
        Some("finished" | "timed_out" | "cancelled")
    ) {
        authorization::remove(id);
        return Ok(());
    }
    anyhow::ensure!(
        matches!(
            watch["status"].as_str(),
            Some("starting" | "running" | "lost")
        ),
        "invalid fresh watch status; unused authorization retained"
    );
    if let Err(error) = authorization::verify(&authorized, &watch, id) {
        authorization::remove(id);
        api.patch(&format!("{path}/{id}"), &json!({"status": "lost",
            "tail": "Remote watch definition differs from the locally authorized command. Nothing was executed."}))?;
        return Err(error);
    }
    let runner = authorized
        .runner_id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let claimed = lease::renew(&fresh, path, id, &runner)?;
    if claimed["active"] != true {
        authorization::remove(id);
        return Ok(());
    }
    if process::stopped() {
        return Ok(());
    }
    let authorized = authorization::take(api, tenant, path, id)?;
    authorization::verify(&authorized, &watch, id)?;
    let output_path = watch_state::directory()?.join(format!("{id}.out"));
    let outcome = execution::run(
        &fresh,
        path,
        id,
        &runner,
        &authorized,
        &output_path,
        claimed["watch"].clone(),
    );
    let tail = tail(&output_path).unwrap_or_default();
    let _ = fs::remove_file(output_path);
    let (status, exit_code) = match outcome {
        Ok(result) => result,
        Err(error) => {
            api.patch(&format!("{path}/{id}"), &json!({"status": "lost", "runner_id": runner,
                "tail": "The local runner failed after claiming execution. The command was not restarted."}))?;
            return Err(error);
        }
    };
    let result = api.patch(
        &format!("{path}/{id}"),
        &json!({
            "status": status, "exit_code": exit_code, "tail": tail, "runner_id": runner,
        }),
    )?;
    if !result["notification"].is_null() {
        let where_text = path.rsplit('/').nth(1).unwrap_or("project");
        let _ = notification_delivery::deliver(&result["notification"], where_text);
    }
    Ok(())
}

fn tail(path: &std::path::Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let length = file.metadata()?.len();
    file.seek(SeekFrom::Start(length.saturating_sub(16000)))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<&str> = text.lines().rev().take(40).collect();
    Ok(lines
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
        .chars()
        .take(10000)
        .collect())
}
