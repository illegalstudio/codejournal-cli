use super::text;
use anyhow::Result;
use rusqlite::{Connection, params};
use serde_json::{Value, json};

pub fn stage(
    db: &Connection,
    session: &Value,
    kind: &str,
    sender: &str,
    detail: &str,
) -> Result<()> {
    if std::env::var("CJ_DELEGATION_JOURNAL").as_deref() == Ok("off") {
        return Ok(());
    }
    let context: Value = serde_json::from_str(text(session, "journal_context"))?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();
    let mut snapshot = json!({});
    for key in [
        "id",
        "target",
        "mode",
        "model",
        "cwd",
        "status",
        "attempt",
        "baseline_sha",
        "integrated_at",
        "exit_code",
        "created_at",
        "updated_at",
    ] {
        snapshot[key] = session[key].clone();
    }
    snapshot["task_title"] = json!(
        text(session, "task")
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(200)
            .collect::<String>()
    );
    for key in ["result", "error"] {
        snapshot[key] = if session[key].is_null() {
            Value::Null
        } else {
            json!(text(session, key).chars().take(500).collect::<String>())
        };
    }
    snapshot["machine"] = context["machine"].clone();
    let mut event = json!({"id": id, "type": "delegation", "kind": kind,
        "session": session["id"], "agent": "delegation", "project": context["project"],
        "project_explicit": context["project"].is_string(), "host": context["machine"],
        "cwd": session["cwd"], "ts": now.to_rfc3339(), "delegation_id": session["id"],
        "delegation": snapshot, "sender": sender, "detail": detail.chars().take(300).collect::<String>(),
        "delegation_host": context["host"], "origin": context["binding"]["origin"],
        "origin_unavailable_reason": context["binding"]["origin_unavailable_reason"]});
    crate::hook::delegation_timestamps::normalize(&mut event)?;
    crate::secret_redaction::value(&mut event);
    db.execute(
        "INSERT INTO journal_delivery (id,payload,queued_at) VALUES (?,?,?)",
        params![
            id,
            event.to_string(),
            now.timestamp_nanos_opt().unwrap_or_default().to_string()
        ],
    )?;
    Ok(())
}

pub fn flush(db: &Connection) -> Result<()> {
    let mut statement =
        db.prepare("SELECT id,payload,queued_at FROM journal_delivery ORDER BY rowid LIMIT 100")?;
    let rows = statement
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (id, payload, epoch) in &rows {
        let event: Value = serde_json::from_str(payload)?;
        crate::outbox::enqueue_captured(event, id, epoch.parse()?)?;
        db.execute("DELETE FROM journal_delivery WHERE id=?", [id])?;
    }
    Ok(())
}
