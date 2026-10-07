use super::{authorization, launch};
use crate::api::Api;
use crate::{attribution, notification_delivery, output};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

pub fn run(
    api: &Api,
    _server: &str,
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
    if api.token.is_empty() {
        bail!("watch startup requires access to its credential, including offline authorization");
    }
    let cwd = std::env::current_dir()?.canonicalize()?;
    let id = uuid::Uuid::new_v4().to_string();
    authorization::save(
        api,
        tenant,
        path,
        &id,
        &command,
        &cwd.to_string_lossy(),
        timeout,
    )?;
    let result = api.post(
        path,
        &json!({
            "id": id, "managed": true, "title": title,
            "notify_on": if notify_on == "end" { "all" } else { "failure" },
            "timeout": timeout, "command": command, "cwd": cwd,
            "host": attribution::host(), "agent": attribution::agent(agent.as_deref()),
        }),
    );
    let created = match result {
        Ok(value) => value,
        Err(error) => {
            if let Some(queued) = error.downcast_ref::<crate::request_outbox::QueuedWrite>() {
                let receipt = json!({"queued": true, "started": false, "watch_id": id,
                    "resource_id": id, "request_id": queued.0, "next": "Run cj sync on this host with the original account to start this watch."});
                output::emit(
                    &receipt,
                    &format!(
                        "Watch not started. Creation queued: {}. Run cj sync on this host.",
                        queued.0
                    ),
                    json_output,
                )?;
                bail!(
                    "watch not started; creation remains queued with its unused local execution authorization"
                );
            }
            authorization::remove(&id);
            return Err(error);
        }
    };
    let actual = created["watch"]["id"]
        .as_str()
        .context("watch ID missing; unused authorization retained")?;
    if actual != id {
        authorization::remove(&id);
        api.patch(&format!("{path}/{actual}"), &json!({"status": "cancelled"}))?;
        bail!("the server does not support managed watch IDs; no command was executed");
    }
    let mut started = launch::run(api, tenant, project, &id)?;
    let delivery = notification_delivery::available()?;
    started["delivery"] = delivery.clone();
    if json_output {
        return output::json(&started);
    }
    let when = if notify_on == "failure" {
        "fails or times out"
    } else {
        "ends"
    };
    let (safe_title, _) = crate::secret_redaction::text(&title);
    if let Some(notice) = output::masking_notice() {
        crate::stdout::println!("{notice}");
    }
    crate::stdout::println!(
        "Watching {} (pid {}): {safe_title}",
        &id[..8],
        started["pid"]
    );
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
