use anyhow::{Context, Result, ensure};
use directories::ProjectDirs;
use rusqlite::{Connection, OptionalExtension, Row, params, types::ValueRef};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};
mod connection;
pub use connection::open;

pub fn directory() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("com", "illegalstudio", "codejournal")
        .context("cannot find Code Journal state directory")?;
    let path = dirs
        .state_dir()
        .unwrap_or_else(|| dirs.data_local_dir())
        .join("delegations");
    fs::create_dir_all(&path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(path)
}

pub fn row(row: &Row<'_>) -> rusqlite::Result<Value> {
    let mut value = serde_json::Map::new();
    for name in row.as_ref().column_names() {
        let item = match row.get_ref(name)? {
            ValueRef::Null => Value::Null,
            ValueRef::Integer(value) => json!(value),
            ValueRef::Real(value) => json!(value),
            ValueRef::Text(value) => json!(String::from_utf8_lossy(value)),
            ValueRef::Blob(_) => Value::Null,
        };
        value.insert(name.to_owned(), item);
    }
    Ok(Value::Object(value))
}

pub fn get(db: &Connection, id: &str) -> Result<Value> {
    uuid::Uuid::parse_str(id).context("invalid delegation ID")?;
    super::contention::retry("read delegation", || {
        Ok(db
            .query_row("SELECT * FROM delegations WHERE id=?", [id], row)
            .optional()?)
    })?
    .with_context(|| format!("unknown delegation: {id}"))
}

pub fn public(mut value: Value) -> Value {
    if let Some(map) = value.as_object_mut() {
        for key in [
            "attempt_token",
            "journal_context",
            "supervisor_identity",
            "guest_identity",
        ] {
            map.remove(key);
        }
    }
    value
}

pub fn update(db: &Connection, id: &str, changes: Value) -> Result<()> {
    let object = changes.as_object().context("invalid delegation changes")?;
    let mut parameters = Vec::new();
    let mut assignments = Vec::new();
    for (key, value) in object {
        ensure!(
            key.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "invalid field"
        );
        assignments.push(format!("{key}=?"));
        parameters.push(sql_value(value));
    }
    parameters.push(rusqlite::types::Value::Text(id.to_owned()));
    db.execute(
        &format!(
            "UPDATE delegations SET {} WHERE id=?",
            assignments.join(",")
        ),
        rusqlite::params_from_iter(parameters),
    )?;
    Ok(())
}

pub fn sql_value(value: &Value) -> rusqlite::types::Value {
    use rusqlite::types::Value as Sql;
    match value {
        Value::Null => Sql::Null,
        Value::Number(n) if n.is_i64() => Sql::Integer(n.as_i64().unwrap_or_default()),
        Value::Number(n) => Sql::Real(n.as_f64().unwrap_or_default()),
        Value::String(s) => Sql::Text(s.clone()),
        _ => Sql::Text(value.to_string()),
    }
}

pub fn event(
    db: &Connection,
    id: &str,
    sender: &str,
    kind: &str,
    payload: Value,
    reply: Option<i64>,
    key: Option<&str>,
) -> Result<Value> {
    db.execute("INSERT OR IGNORE INTO events (delegation_id,sender,kind,payload,reply_to,idempotency_key,created_at) VALUES (?,?,?,?,?,?,?)",
        params![id, sender, kind, payload.to_string(), reply, key, super::now()])?;
    let mut value = if let Some(key) = key {
        db.query_row(
            "SELECT * FROM events WHERE delegation_id=? AND sender=? AND idempotency_key=?",
            params![id, sender, key],
            row,
        )?
    } else {
        db.query_row(
            "SELECT * FROM events WHERE seq=?",
            [db.last_insert_rowid()],
            row,
        )?
    };
    value["payload"] = serde_json::from_str(super::text(&value, "payload"))?;
    Ok(value)
}

pub fn transaction<T>(operation: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
    let db = open()?;
    let tx = super::contention::retry("begin transaction", || {
        Ok(rusqlite::Transaction::new_unchecked(
            &db,
            rusqlite::TransactionBehavior::Immediate,
        )?)
    })?;
    let result = operation(&tx)?;
    // Retry COMMIT on the same transaction without replaying its callback.
    super::contention::retry("commit transaction", || Ok(tx.execute_batch("COMMIT")?))?;
    drop(tx);
    if let Err(error) = super::delivery::flush(&db) {
        eprintln!("Delegation journal delivery remains pending: {error}");
    }
    Ok(result)
}
