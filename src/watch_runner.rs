use crate::api::Api;
use crate::watch_state;
use anyhow::{Context, Result};
use serde_json::json;
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub fn run(api: &Api, path: &str, id: &str) -> Result<()> {
    let watch = api.get(&format!("{path}/{id}"))?["watch"].clone();
    if watch["status"] != "running" {
        return Ok(());
    }
    let command: Vec<String> =
        serde_json::from_str(watch["command"].as_str().context("watch command missing")?)?;
    let cwd = watch["cwd"].as_str().context("watch cwd missing")?;
    let output_path = watch_state::directory()?.join(format!("{id}.out"));
    let outcome = execute(&command, cwd, watch["timeout"].as_u64(), &output_path);
    let tail = tail(&output_path).unwrap_or_default();
    let _ = fs::remove_file(output_path);
    for _ in 0..20 {
        if watch_state::pid_path(id)?.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    watch_state::remove_pid(id);
    let (status, exit_code) = outcome?;
    api.patch(
        &format!("{path}/{id}"),
        &json!({
            "status": status, "exit_code": exit_code, "tail": tail,
        }),
    )?;
    Ok(())
}

fn execute(
    command: &[String],
    cwd: &str,
    timeout: Option<u64>,
    output_path: &std::path::Path,
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
    let spawned = Command::new(&command[0])
        .args(&command[1..])
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::from(file))
        .stderr(Stdio::from(stderr))
        .spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(error) => {
            fs::write(output_path, format!("could not start: {error}\n"))?;
            return Ok(("finished", 127));
        }
    };
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(("finished", status.code().unwrap_or(1)));
        }
        if timeout.is_some_and(|seconds| started.elapsed() >= Duration::from_secs(seconds)) {
            child.kill()?;
            let _ = child.wait();
            return Ok(("timed_out", 124));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
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
