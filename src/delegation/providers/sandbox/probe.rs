use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::{
    collections::VecDeque,
    io::Read,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub fn check(wrapper: &Command, session: &Value) -> Result<()> {
    let mut command = Command::new(wrapper.get_program());
    command
        .args(wrapper.get_args())
        .arg(std::env::current_exe()?)
        .arg("--version")
        .current_dir(super::super::super::text(session, "cwd"))
        .stdin(Stdio::null())
        .env_remove("CJ_DELEGATION_TOKEN")
        .env_remove("CJ_TOKEN")
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    super::super::super::process::configure(&mut command);
    let mut child = command.spawn().context("cannot start sandbox preflight")?;
    let mut stderr = child
        .stderr
        .take()
        .context("sandbox diagnostics unavailable")?;
    let reader = std::thread::spawn(move || {
        let mut tail = VecDeque::with_capacity(2048);
        let mut buffer = [0_u8; 2048];
        while let Ok(count) = stderr.read(&mut buffer) {
            if count == 0 {
                break;
            }
            tail.extend(&buffer[..count]);
            while tail.len() > 2048 {
                tail.pop_front();
            }
        }
        String::from_utf8_lossy(&tail.into_iter().collect::<Vec<_>>()).into_owned()
    });
    let deadline = Instant::now() + Duration::from_millis(1000);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let detail = if reader.is_finished() {
                    reader.join().unwrap_or_default()
                } else {
                    String::new()
                };
                if status.success() {
                    return Ok(());
                }
                bail!("sandbox preflight failed ({status}): {}", detail.trim());
            }
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            other => {
                super::super::super::process::stop(&mut child);
                match other {
                    Err(error) => return Err(error).context("cannot inspect sandbox preflight"),
                    _ => bail!("sandbox preflight timed out"),
                }
            }
        }
    }
}
