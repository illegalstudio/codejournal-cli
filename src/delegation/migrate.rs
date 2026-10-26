use super::{context, process, store};
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OpenFlags};
use serde_json::{Value, json};
use std::path::Path;
mod mapping;

pub fn run(path: &Path, merge: bool) -> Result<Value> {
    context::host_only()?;
    let source = path.canonicalize()?;
    let target = store::directory()?.join("delegations.sqlite3");
    ensure!(
        target.canonicalize().ok().as_ref() != Some(&source),
        "source database must be separate from the native store"
    );
    #[cfg(unix)]
    if let Ok(target) = target.metadata() {
        use std::os::unix::fs::MetadataExt;
        let original = source.metadata()?;
        ensure!(
            target.dev() != original.dev() || target.ino() != original.ino(),
            "source database must not alias the native store"
        );
    }
    let mut original = Connection::open_with_flags(&source, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let read = original.transaction()?;
    let rows = read
        .prepare("SELECT * FROM delegations")?
        .query_map([], store::row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for row in &rows {
        ensure!(
            !process::alive(&row["supervisor_pid"]) && !process::alive(&row["guest_pid"]),
            "legacy delegation {} still has a live process; finish it before migration",
            row["id"]
        );
    }
    let events = read
        .prepare("SELECT * FROM events ORDER BY seq")?
        .query_map([], store::row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    store::transaction(|db| {
        db.execute_batch("CREATE TABLE IF NOT EXISTS legacy_imports (source TEXT PRIMARY KEY, imported_at REAL NOT NULL)")?;
        let name = source
            .to_str()
            .context("legacy database path is not UTF-8")?;
        let imported: i64 = db.query_row(
            "SELECT count(*) FROM legacy_imports WHERE source=?",
            [name],
            |r| r.get(0),
        )?;
        if imported > 0 {
            return Ok(json!({"already_imported":true}));
        }
        let offset = mapping::offset(db, &rows, merge)?;
        for row in &rows {
            let mut session = row.clone();
            for key in [
                "supervisor_pid",
                "guest_pid",
                "supervisor_identity",
                "guest_identity",
            ] {
                session[key] = Value::Null;
            }
            for key in ["host_ack_seq", "guest_ack_seq"] {
                mapping::translate(&mut session, key, offset)?;
            }
            insert(db, "delegations", &session)?;
        }
        for event in &events {
            let mut event = event.clone();
            mapping::translate(&mut event, "seq", offset)?;
            mapping::translate(&mut event, "reply_to", offset)?;
            insert(db, "events", &event)?;
        }
        db.execute(
            "INSERT INTO legacy_imports (source,imported_at) VALUES (?,?)",
            rusqlite::params![name, super::now()],
        )?;
        Ok(
            json!({"imported_sessions":rows.len(),"imported_events":events.len(),"sequence_offset":offset,"original_unchanged":true,
            "journal_binding":"Existing history has no inferred account binding; future recovery events remain unbound."}),
        )
    })
}

fn insert(db: &Connection, table: &str, row: &Value) -> Result<()> {
    let map = row.as_object().context("invalid legacy row")?;
    let mut statement = db.prepare(&format!("PRAGMA table_info({table})"))?;
    let allowed = statement
        .query_map([], |r| r.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let fields = map
        .keys()
        .filter(|key| allowed.contains(key))
        .collect::<Vec<_>>();
    let names = fields
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let placeholders = vec!["?"; fields.len()].join(",");
    let values = fields
        .iter()
        .map(|key| store::sql_value(&map[*key]))
        .collect::<Vec<_>>();
    db.execute(
        &format!("INSERT INTO {table} ({names}) VALUES ({placeholders})"),
        rusqlite::params_from_iter(values),
    )?;
    Ok(())
}
