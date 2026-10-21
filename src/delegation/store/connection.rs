use super::super::{contention, schema};
use anyhow::{Context, Result};
use rusqlite::{Connection, TransactionBehavior};
use std::{fs, time::Duration};

pub fn open() -> Result<Connection> {
    let path = super::directory()?.join("delegations.sqlite3");
    let mut db = Connection::open(&path).context("open delegation database")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    db.busy_timeout(Duration::from_millis(250))?;
    contention::retry("configure connection", || {
        Ok(db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;")?)
    })?;
    let version: i64 = contention::retry("read schema version", || {
        Ok(db.pragma_query_value(None, "user_version", |row| row.get(0))?)
    })?;
    if version == 0 {
        contention::retry("initialize schema", || initialize(&mut db))?;
    }
    Ok(db)
}

fn initialize(db: &mut Connection) -> Result<()> {
    let mode: String = db.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
    if mode != "wal" {
        db.pragma_update(None, "journal_mode", "WAL")?;
    }
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let version: i64 = tx.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version == 0 {
        tx.execute_batch(schema::SQL)?;
        tx.pragma_update(None, "user_version", 1)?;
    }
    Ok(tx.commit()?)
}
