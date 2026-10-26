use super::{store, text};
use anyhow::Result;
use rusqlite::{Connection, params};
use serde_json::Value;

pub fn list(db: &Connection, id: &str, after: i64, other_than: Option<&str>) -> Result<Vec<Value>> {
    let mut statement = db.prepare("SELECT * FROM events WHERE delegation_id=? AND seq>? AND (? IS NULL OR sender!=?) ORDER BY seq LIMIT 500")?;
    let mut rows = statement.query(params![id, after, other_than, other_than])?;
    let mut events = Vec::new();
    while let Some(row) = rows.next()? {
        let mut event = store::row(row)?;
        event["payload"] = serde_json::from_str(text(&event, "payload"))?;
        events.push(event);
    }
    Ok(events)
}

pub fn last(db: &Connection, id: &str, kind: &str) -> Result<Option<Value>> {
    let mut statement = db.prepare(
        "SELECT * FROM events WHERE delegation_id=? AND kind=? ORDER BY seq DESC LIMIT 1",
    )?;
    let mut rows = statement.query(params![id, kind])?;
    if let Some(row) = rows.next()? {
        let mut event = store::row(row)?;
        event["payload"] = serde_json::from_str(text(&event, "payload"))?;
        Ok(Some(event))
    } else {
        Ok(None)
    }
}
