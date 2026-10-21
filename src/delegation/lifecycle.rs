mod attempts;
mod integration;
mod reconcile;
pub use attempts::{attach, finish, prepare};
pub use integration::run as integrate;
pub use reconcile::reconcile;

use super::{context, delivery, process, store, text};
use anyhow::{Result, ensure};
use serde_json::{Value, json};

pub fn cancel(id: &str, reason: &str) -> Result<Value> {
    context::host_only()?;
    let session = store::transaction(|db| {
        let session = store::get(db, id)?;
        ensure!(
            !super::terminal(&session),
            "cannot cancel a terminal delegation"
        );
        store::update(
            db,
            id,
            json!({"status": "cancelled", "error": reason, "updated_at": super::now()}),
        )?;
        store::event(
            db,
            id,
            "system",
            "cancelled",
            json!({"reason": reason}),
            None,
            None,
        )?;
        let updated = store::get(db, id)?;
        delivery::stage(db, &updated, "cancelled", "host", reason)?;
        Ok(updated)
    })?;
    if let Some(pid) = session["supervisor_pid"]
        .as_u64()
        .and_then(|p| u32::try_from(p).ok())
    {
        process::terminate(pid, text(&session, "supervisor_identity"));
    }
    // A missing supervisor must not leave its verified guest group running.
    if !process::matches(&session["supervisor_pid"], &session["supervisor_identity"]) {
        if let Some(pid) = session["guest_pid"]
            .as_u64()
            .and_then(|p| u32::try_from(p).ok())
        {
            process::terminate_group(pid, text(&session, "guest_identity"));
        }
    }
    Ok(json!({"session": store::public(session)}))
}
