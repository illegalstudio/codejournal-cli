use super::{context, lifecycle, store, text};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::{
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

pub fn output(id: &str, lines: u16) -> Result<Value> {
    context::host_only()?;
    ensure!(
        (1..=200).contains(&lines),
        "diagnostic lines must be between 1 and 200"
    );
    let session = lifecycle::refresh(id)?;
    let directory = store::directory()?;
    let mut value = json!({"id":id, "status":session["status"], "attempt":session["attempt"],
        "source":"local", "provider_output_at":session["provider_output_at"],
        "last_guest_activity_at":session["last_activity"], "contact_at":session["guest_heartbeat"],
        "progress_state":if session["status"] == "waiting_for_host" {"waiting_for_host"} else if session["provider_output_at"].is_null() {"no_observed_provider_output"} else {"provider_output_observed"},
        "note":"Heartbeat is contact, not progress. Silence does not identify an API wait or a stuck provider.",
        "capture_limit_bytes":2097152, "streams":{}});
    for stream in ["stdout", "stderr"] {
        let path = directory.join(format!("{id}-attempt-{}.{stream}.log", session["attempt"]));
        value["streams"][stream] = tail(&path, lines)?;
        if let Some(content) = value["streams"][stream]["text"].as_str() {
            let token = text(&session, "attempt_token");
            if !token.is_empty() {
                value["streams"][stream]["text"] = json!(content.replace(token, "[redacted]"));
            }
        }
    }
    crate::secret_redaction::value(&mut value);
    Ok(value)
}

fn tail(path: &Path, lines: u16) -> Result<Value> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    if let Ok(metadata) = path.symlink_metadata() {
        ensure!(
            metadata.is_file() && !metadata.file_type().is_symlink(),
            "diagnostic output must be a regular file"
        );
    }
    let mut file = match options.open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(json!({"available":false,"text":""}));
        }
        Err(error) => return Err(error.into()),
    };
    let size = file.metadata()?.len();
    ensure!(
        file.metadata()?.is_file(),
        "diagnostic output must be a regular file"
    );
    file.seek(SeekFrom::Start(size.saturating_sub(16384)))?;
    let mut bytes = Vec::new();
    file.take(16384).read_to_end(&mut bytes)?;
    let content = String::from_utf8_lossy(&bytes);
    let mut selected = content
        .lines()
        .rev()
        .take(usize::from(lines))
        .collect::<Vec<_>>();
    selected.reverse();
    Ok(
        json!({"available":true,"text":selected.join("\n"), "capture_truncated":size>=2097152,
        "tail_truncated":size>bytes.len() as u64 || content.lines().count()>usize::from(lines)}),
    )
}
