use crate::{outbox, session_state};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

pub fn run(raw: &str) -> Result<()> {
    let operation: Value = serde_json::from_str(raw).context("session-record expects JSON")?;
    if operation["op"] != "session_start" {
        bail!("session-record expects a session_start operation");
    }
    let session = &operation["session"];
    let key = session["id"].as_str().context("session ID missing")?;
    let project = operation["project"]["slug"].as_str();
    let ts = session["started_at"]
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(session_state::now);
    let event = json!({
        "type": "start", "session": key, "agent": session["agent"].as_str().unwrap_or("unknown"),
        "project": project, "host": session["host"], "cwd": session["checkout_path"],
        "ts": ts,
        "source": "legacy-session-record",
    });
    outbox::enqueue(event)?;
    let _ = outbox::spawn_flush();
    Ok(())
}
