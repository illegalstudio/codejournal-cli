use crate::api::Api;
use crate::watch_args::WatchAction;
use crate::{
    attribution, notification_delivery, output, project_bootstrap, watch_runner, watch_state,
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
    let project = if project_name.is_none() && matches!(&action, WatchAction::Start { .. }) {
        project_bootstrap::ensure(api, tenant, false)?
    } else {
        project_bootstrap::resolved_slug(api, tenant, project_name)?
    };
    let path = format!("{}/watches", endpoint(tenant, &project));
    match action {
        WatchAction::Start {
            title,
            notify_on,
            timeout,
            agent,
            command,
        } => {
            let cwd = std::env::current_dir()?.canonicalize()?;
            let host = std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_owned());
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
                    &project,
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
                println!("Watching {} (pid {}): {title}", &id[..8], child.id());
                let channels = delivery["channels"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("When it {when}, a notification goes to: {channels}.");
                if let Some(note) = delivery["note"].as_str() {
                    println!("note: {note}.");
                }
                Ok(())
            }
        }
        WatchAction::List { all } => {
            output::json(&api.get(&format!("{path}?all={}", if all { 1 } else { 0 }))?)
        }
        WatchAction::Cancel { id } => {
            let watch = resolve(api, &path, &id)?;
            let full_id = watch["id"].as_str().context("watch ID missing")?;
            if watch["status"] != "running" {
                return output::json(&json!({"watch": watch, "cancelled": false}));
            }
            let pid = watch_state::read_pid(full_id)
                .context("this watch is not running on this machine")?;
            let result = api.patch(
                &format!("{path}/{full_id}"),
                &json!({"status": "cancelled"}),
            )?;
            terminate(pid);
            watch_state::remove_pid(full_id);
            output::json(&result)
        }
        WatchAction::Run { id } => watch_runner::run(api, &path, &id),
    }
}

fn resolve(api: &Api, path: &str, prefix: &str) -> Result<Value> {
    let listing = api.get(&format!("{path}?all=1"))?;
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
