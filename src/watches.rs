use crate::api::Api;
use crate::watch_args::WatchAction;
use crate::{
    attribution, notification_delivery, output, project_bootstrap, watch_format, watch_runner,
    watch_state,
};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::process::{Command, Stdio};

pub fn endpoint(tenant: &str, project_name: &str) -> String {
    format!("/api/v1/tenants/{tenant}/projects/{project_name}")
}

pub fn run(
    api: &Api,
    server: &str,
    tenant: &str,
    project_name: Option<&str>,
    action: WatchAction,
    json_output: bool,
) -> Result<()> {
    let global = matches!(
        &action,
        WatchAction::List { .. } | WatchAction::Cancel { .. }
    );
    let project = if global {
        None
    } else if project_name.is_none() && matches!(&action, WatchAction::Start { .. }) {
        Some(project_bootstrap::ensure(api, tenant, false)?)
    } else {
        Some(project_bootstrap::resolved_slug(api, tenant, project_name)?)
    };
    let path = project.as_ref().map_or_else(
        || format!("/api/v1/tenants/{tenant}/watches"),
        |slug| format!("{}/watches", endpoint(tenant, slug)),
    );
    match action {
        WatchAction::Start {
            title,
            notify_on,
            timeout,
            agent,
            command,
        } => {
            let cwd = std::env::current_dir()?.canonicalize()?;
            let host = attribution::host();
            let created = api.post(
                &path,
                &json!({
                    "title": title, "notify_on": if notify_on == "end" { "all" } else { "failure" }, "timeout": timeout,
                    "command": command, "cwd": cwd, "host": host,
                    "agent": attribution::agent(agent.as_deref()),
                }),
            )?;
            let id = created["watch"]["id"]
                .as_str()
                .context("watch ID missing")?;
            let executable = std::env::current_exe()?;
            let mut worker = Command::new(executable);
            worker
                .args([
                    "--server",
                    server,
                    "--project",
                    project.as_deref().unwrap_or(""),
                    "watch",
                    "run",
                    id,
                ])
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
        WatchAction::List { all } => {
            let result = api.get(&local_list_path(&path, all)?)?;
            output::emit(&result, &watch_format::list(&result), json_output)
        }
        WatchAction::Cancel { id } => {
            let watch = resolve(api, &path, &id)?;
            let full_id = watch["id"].as_str().context("watch ID missing")?;
            if watch["status"] != "running" {
                let text = watch_format::cancel(&watch, false);
                return output::emit(
                    &json!({"watch": watch, "cancelled": false}),
                    &text,
                    json_output,
                );
            }
            let pid = watch_state::read_pid(full_id)
                .context("this watch is not running on this machine")?;
            let watch_project = watch["project_slug"]
                .as_str()
                .context("watch project missing")?;
            let result = api.patch(
                &format!("{}/watches/{full_id}", endpoint(tenant, watch_project)),
                &json!({"status": "cancelled"}),
            )?;
            terminate(pid);
            watch_state::remove_pid(full_id);
            let updated = &result["watch"];
            output::emit(
                &json!({"watch": updated, "cancelled": true}),
                &watch_format::cancel(updated, true),
                json_output,
            )
        }
        WatchAction::Run { id } => watch_runner::run(api, &path, &id),
    }
}

fn resolve(api: &Api, path: &str, prefix: &str) -> Result<Value> {
    let listing = api.get(&local_list_path(path, true)?)?;
    let watches = listing["watches"]
        .as_array()
        .context("invalid watch listing")?;
    let matches: Vec<_> = watches
        .iter()
        .filter(|watch| {
            watch["id"]
                .as_str()
                .is_some_and(|id| id.replace('-', "").starts_with(&prefix.replace('-', "")))
        })
        .collect();
    match matches.as_slice() {
        [watch] => Ok((*watch).clone()),
        [] => bail!("no watch matches {prefix}"),
        _ => bail!("watch prefix {prefix} is ambiguous"),
    }
}

fn local_list_path(path: &str, all: bool) -> Result<String> {
    let host = attribution::host();
    let query = reqwest::Url::parse_with_params(
        "http://local/",
        [
            ("all", if all { "1" } else { "0" }),
            ("host", host.as_str()),
        ],
    )?;
    Ok(format!("{path}?{}", query.query().unwrap_or("")))
}

fn terminate(pid: u32) {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(pid as i32), libc::SIGTERM);
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status();
    }
}
