use super::authorization;
use crate::api::Api;
use crate::{attribution, notification_delivery, output, watch_state};
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::process::{Command, Stdio};

pub fn run(
    api: &Api,
    server: &str,
    tenant: &str,
    project: &str,
    path: &str,
    title: String,
    notify_on: String,
    timeout: Option<u64>,
    agent: Option<String>,
    command: Vec<String>,
    json_output: bool,
) -> Result<()> {
    let cwd = std::env::current_dir()?.canonicalize()?;
    let host = attribution::host();
    let created = api.post(
        path,
        &json!({
            "title": title, "notify_on": if notify_on == "end" { "all" } else { "failure" }, "timeout": timeout,
            "command": command, "cwd": cwd, "host": host,
            "agent": attribution::agent(agent.as_deref()),
        }),
    )?;
    let id = created["watch"]["id"]
        .as_str()
        .context("watch ID missing")?;
    authorization::save(
        api,
        tenant,
        path,
        id,
        &command,
        &cwd.to_string_lossy(),
        timeout,
    )?;
    let executable = std::env::current_exe()?;
    let mut worker = Command::new(executable);
    worker
        .args(["--server", server, "--project", project, "watch", "run", id])
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        worker.process_group(0);
    }
    let spawned = worker.spawn();
    let child = match spawned {
        Ok(child) => child,
        Err(error) => {
            authorization::remove(id);
            api.patch(&format!("{path}/{id}"), &json!({"status": "cancelled"}))?;
            return Err(error.into());
        }
    };
    watch_state::write_pid(id, child.id())?;
    let delivery = notification_delivery::available()?;
    if json_output {
        output::json(&json!({"watch": created["watch"], "pid": child.id(),
            "delivery": delivery}))
    } else {
        let when = if notify_on == "failure" {
            "fails or times out"
        } else {
            "ends"
        };
        let (safe_title, _) = crate::secret_redaction::text(&title);
        if let Some(notice) = output::masking_notice() {
            crate::stdout::println!("{notice}");
        }
        crate::stdout::println!("Watching {} (pid {}): {safe_title}", &id[..8], child.id());
        let channels = delivery["channels"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        crate::stdout::println!("When it {when}, a notification goes to: {channels}.");
        if let Some(note) = delivery["note"].as_str() {
            crate::stdout::println!("note: {note}.");
        }
        Ok(())
    }
}
