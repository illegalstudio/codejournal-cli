use super::{authorization::Authorization, lease, process};
use crate::{api::Api, watch_state};
use anyhow::Result;
use serde_json::{Value, json};
use std::fs::OpenOptions;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct Running(Child, bool);
impl Drop for Running {
    fn drop(&mut self) {
        if !self.1 {
            let _ = process::kill_tree(&mut self.0);
        }
    }
}

pub fn run(
    api: &Api,
    path: &str,
    id: &str,
    runner: &str,
    authorized: &Authorization,
    output_path: &Path,
    watch: Value,
) -> Result<(&'static str, i32)> {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(output_path)?;
    let stderr = file.try_clone()?;
    let mut command = Command::new(&authorized.command[0]);
    command
        .args(&authorized.command[1..])
        .current_dir(&authorized.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::from(file))
        .stderr(Stdio::from(stderr));
    process::group(&mut command);
    let mut child = match command.spawn() {
        Ok(child) => Running(child, false),
        Err(error) => {
            std::fs::write(output_path, format!("could not start: {error}\n"))?;
            return Ok(("finished", 127));
        }
    };
    watch_state::write_json(
        &watch_state::pid_path(id)?.with_extension("ready.json"),
        &json!({"watch": watch, "pid": std::process::id(), "started": true}),
    )?;
    let started = Instant::now();
    let mut heartbeat = Instant::now();
    loop {
        if let Some(status) = child.0.try_wait()? {
            child.1 = true;
            return Ok(("finished", status.code().unwrap_or(1)));
        }
        if process::stopped() {
            return Ok(("cancelled", 0));
        }
        if authorized
            .timeout
            .is_some_and(|seconds| started.elapsed() >= Duration::from_secs(seconds))
        {
            return Ok(("timed_out", 124));
        }
        if heartbeat.elapsed() >= Duration::from_secs(30) {
            match lease::renew(api, path, id, runner) {
                Ok(value) if value["active"] != true => return Ok(("cancelled", 0)),
                Ok(_) => {}
                Err(_) => eprintln!(
                    "Watch heartbeat could not be delivered; the command remains active locally."
                ),
            }
            heartbeat = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
