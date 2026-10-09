use super::{authorization, process};
use crate::{api::Api, watch_state};
use anyhow::{Context, Result, bail};
use fs2::FileExt;
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub fn run(api: &Api, tenant: &str, project: &str, id: &str) -> Result<Value> {
    let lock = OpenOptions::new()
        .write(true)
        .create(true)
        .open(watch_state::pid_path(id)?.with_extension("launch.lock"))?;
    lock.try_lock_exclusive()
        .context("watch startup is already in progress")?;
    let scope = crate::outbox::fingerprint(api.server(), tenant, &api.token);
    if let Ok(pid) = watch_state::read_pid(id, &scope)
        && process::owns(pid, id)
    {
        bail!("watch runner is already active");
    }
    let path = format!("{}/watches", super::endpoint(tenant, project));
    let authorized = authorization::load(api, tenant, &path, id)?;
    let ready = watch_state::pid_path(id)?.with_extension("ready.json");
    let failure = watch_state::pid_path(id)?.with_extension("failure.json");
    let _ = fs::remove_file(&ready);
    let _ = fs::remove_file(&failure);
    let log = watch_state::pid_path(id)?.with_extension("runner.log");
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut worker = Command::new(std::env::current_exe()?);
    worker
        .args([
            "--server",
            api.server(),
            "--project",
            project,
            "watch",
            "run",
            id,
        ])
        .current_dir(&authorized.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(options.open(&log)?));
    process::detached(&mut worker);
    let mut child = worker
        .spawn()
        .context("could not spawn watch runner; authorization retained for cj sync")?;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(bytes) = fs::read(&ready) {
            let result = serde_json::from_slice(&bytes)?;
            fs::remove_file(&ready)?;
            return Ok(result);
        }
        if child.try_wait()?.is_some() {
            break;
        }
        if Instant::now() >= deadline {
            process::terminate(child.id(), id);
            let grace = Instant::now() + Duration::from_secs(3);
            while child.try_wait()?.is_none() && Instant::now() < grace {
                std::thread::sleep(Duration::from_millis(20));
            }
            if child.try_wait()?.is_none() {
                let _ = child.kill();
            }
            let _ = child.wait();
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let diagnostic = fs::read(&failure)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|value| value["error"].as_str().map(str::to_owned))
        .map(|error| format!(": {error}"))
        .unwrap_or_default();
    let _ = fs::remove_file(failure);
    bail!(
        "watch did not start{diagnostic}; inspect {}. Any unused local authorization is retained for cj sync",
        log.display()
    )
}
