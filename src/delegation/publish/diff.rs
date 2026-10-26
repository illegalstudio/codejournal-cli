use super::super::{context, store, text, workspace};
use anyhow::{Result, ensure};
use rusqlite::params;

pub fn inspect(id: &str, role: &str, kind: &str, key: Option<&str>) -> Result<Option<String>> {
    if role != "guest" || !matches!(kind, "checkpoint" | "result") {
        return Ok(None);
    }
    let db = store::open()?;
    let session = store::get(&db, id)?;
    context::authorize(&session, role)?;
    if text(&session, "mode") != "write" || super::super::terminal(&session) {
        return Ok(None);
    }
    if let Some(key) = key {
        let exists: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM events WHERE delegation_id=? AND sender=? AND idempotency_key=?)",
            params![id, role, key],
            |row| row.get(0),
        )?;
        if exists {
            return Ok(None);
        }
    }
    drop(db);
    let changed = workspace::changed(&session, true)?;
    ensure!(
        !changed.is_empty(),
        "write checkpoint or result requires a workspace diff"
    );
    Ok(Some(changed))
}
