use crate::{hook_session, outbox, project, session_state};
use anyhow::Result;
use serde_json::Value;
use std::io::Read;
use std::path::PathBuf;

pub fn run(event: &str, kind: Option<&str>) -> Result<()> {
    if std::env::var("CODE_JOURNAL_HOOKS").as_deref() == Ok("off") {
        return Ok(());
    }
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let payload: Value = serde_json::from_str(&input).unwrap_or(Value::Null);
    if event == "delegation" {
        return delegation(kind.unwrap_or("updated"), &payload);
    }
    let Some(session) = payload["session_id"]
        .as_str()
        .or_else(|| payload["conversation_id"].as_str())
        .map(str::to_owned)
        .or_else(session_state::current_id)
    else {
        return Ok(());
    };
    let cwd = payload["cwd"]
        .as_str()
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?);
    std::env::set_current_dir(&cwd)?;
    hook_session::handle(event, &payload, &session, &cwd)
}

fn delegation(kind: &str, payload: &Value) -> Result<()> {
    let Some(snapshot) = payload["delegation"].as_object() else {
        return Ok(());
    };
    let Some(id) = snapshot.get("id").and_then(Value::as_str) else {
        return Ok(());
    };
    if uuid::Uuid::parse_str(id).is_err() {
        return Ok(());
    }
    let cwd = snapshot.get("cwd").and_then(Value::as_str).unwrap_or("");
    let project = payload["project"].as_str().map(str::to_owned).or_else(|| {
        std::env::set_current_dir(cwd).ok()?;
        project::slug(None).ok()
    });
    let event = serde_json::json!({"type": "delegation", "kind": kind,
        "session": id, "agent": "delegation", "project": project,
        "project_explicit": payload["project"].as_str().is_some(),
        "host": payload["machine"].as_str(), "cwd": cwd,
        "ts": payload["at"].as_str().map(str::to_owned)
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
        "delegation_id": id, "delegation": snapshot, "sender": payload["sender"],
        "detail": payload["detail"], "delegation_host": payload["host"]});
    outbox::enqueue(event)?;
    outbox::spawn_flush()
}
