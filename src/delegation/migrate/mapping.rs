use anyhow::{Context, Result, ensure};
use rusqlite::Connection;
use serde_json::{Value, json};

pub fn offset(db: &Connection, sessions: &[Value], merge: bool) -> Result<i64> {
    let count: i64 = db.query_row("SELECT count(*) FROM delegations", [], |r| r.get(0))?;
    ensure!(
        count == 0 || merge,
        "legacy migration requires an empty delegation store or explicit --merge; no existing sessions were changed"
    );
    for session in sessions {
        let exists: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM delegations WHERE id=?)",
            [super::super::text(session, "id")],
            |r| r.get(0),
        )?;
        ensure!(
            !exists,
            "legacy session {} already exists; no existing sessions were changed",
            session["id"]
        );
    }
    Ok(db.query_row("SELECT coalesce(max(seq),0) FROM events", [], |r| r.get(0))?)
}

pub fn translate(row: &mut Value, field: &str, offset: i64) -> Result<()> {
    if row[field].is_null() {
        return Ok(());
    }
    let value = row[field]
        .as_i64()
        .with_context(|| format!("invalid legacy cursor: {field}"))?;
    ensure!(value >= 0, "legacy cursor must be nonnegative");
    row[field] = json!(
        value
            .checked_add(offset)
            .context("legacy cursor overflow")?
    );
    Ok(())
}
