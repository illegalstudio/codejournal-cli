use super::super::{delivery, process, store, text};
use anyhow::Result;
use rusqlite::Connection;
use serde_json::{Value, json};

pub fn needed(session: &Value) -> bool {
    if !matches!(
        text(session, "status"),
        "starting" | "recovering" | "running" | "waiting_for_host" | "waiting_for_guest"
    ) || process::matches(&session["supervisor_pid"], &session["supervisor_identity"])
    {
        return false;
    }
    let guest_alive = process::matches(&session["guest_pid"], &session["guest_identity"]);
    let contact = session["guest_heartbeat"]
        .as_f64()
        .or_else(|| session["updated_at"].as_f64())
        .unwrap_or_default();
    if super::super::now() - contact < if guest_alive { 30.0 } else { 5.0 } {
        return false;
    }
    true
}

pub fn refresh(id: &str) -> Result<Value> {
    let session = store::get(&store::open()?, id)?;
    if !needed(&session) {
        return Ok(session);
    }
    store::transaction(|db| reconcile(db, store::get(db, id)?))
}

pub fn reconcile(db: &Connection, session: Value) -> Result<Value> {
    if !needed(&session) {
        return Ok(session);
    }
    let guest_alive = process::matches(&session["guest_pid"], &session["guest_identity"]);
    let id = text(&session, "id");
    let reason = if guest_alive {
        "supervisor is unavailable but the guest remains alive; cancel the confirmed delegation before recovery"
    } else {
        "supervisor and guest processes are no longer available; use cj delegation recover"
    };
    store::event(
        db,
        id,
        "system",
        "process_lost",
        json!({"reason":reason,"attempt":session["attempt"]}),
        None,
        Some(&format!("process-lost-{}", session["attempt"])),
    )?;
    store::update(
        db,
        id,
        json!({"status":"stalled","error":reason,"supervisor_pid":null,
        "guest_pid":if guest_alive {session["guest_pid"].clone()} else {Value::Null},"updated_at":super::super::now()}),
    )?;
    let updated = store::get(db, id)?;
    delivery::stage(db, &updated, "stalled", "system", reason)?;
    Ok(updated)
}
