use super::{context, lifecycle, process, store, text};
use anyhow::{Result, ensure};
use serde_json::{Value, json};

pub fn list(active: bool, limit: u16, cwd_scoped: bool) -> Result<Value> {
    context::host_only()?;
    let cwd = if cwd_scoped {
        Some(
            std::env::current_dir()?
                .canonicalize()?
                .to_string_lossy()
                .into_owned(),
        )
    } else {
        None
    };
    let records = store::read(|db| {
        let mut statement=db.prepare("SELECT * FROM delegations WHERE (?=0 OR status NOT IN ('completed','failed','cancelled') OR (mode='write' AND status='completed' AND integrated_at IS NULL)) AND (? IS NULL OR cwd=?) ORDER BY updated_at DESC LIMIT ?")?;
        let records = statement
            .query_map(rusqlite::params![active, cwd, cwd, limit], store::row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(records)
    })?;
    let mut sessions = Vec::new();
    for record in records {
        sessions.push(store::public(lifecycle::refresh(text(&record, "id"))?));
    }
    Ok(
        json!({"sessions":sessions, "scope":if cwd_scoped {"cwd"} else {"machine"}, "cwd_filter":cwd}),
    )
}

pub fn purge(id: &str, force: bool) -> Result<Value> {
    context::host_only()?;
    store::transaction(|db| {
        let session = store::get(db, id)?;
        ensure!(
            !process::owns_processes(&session),
            "cannot purge a delegation with live processes; cancel it first"
        );
        ensure!(
            force
                || (super::terminal(&session)
                    && !(text(&session, "mode") == "write"
                        && text(&session, "status") == "completed"
                        && session["integrated_at"].is_null())),
            "refusing to purge an unresolved delegation without --force"
        );
        db.execute("DELETE FROM delegations WHERE id=?", [id])?;
        Ok(())
    })?;
    for entry in std::fs::read_dir(store::directory()?)? {
        let path = entry?.path();
        if path.is_file()
            && path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with(&format!("{id}-")))
        {
            std::fs::remove_file(path)?;
        }
    }
    Ok(json!({"purged":id}))
}

pub fn gc(days: f64) -> Result<Value> {
    context::host_only()?;
    ensure!(
        days.is_finite() && days >= 0.0,
        "retention days must be finite and nonnegative"
    );
    let db = store::open()?;
    let mut statement=db.prepare("SELECT id FROM delegations WHERE status IN ('completed','failed','cancelled') AND NOT (mode='write' AND status='completed' AND integrated_at IS NULL) AND updated_at<?")?;
    let ids = statement
        .query_map([super::now() - days * 86400.0], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut purged = Vec::new();
    for id in ids {
        purge(&id, false)?;
        purged.push(id);
    }
    Ok(json!({"purged":purged,"retention_days":days}))
}
