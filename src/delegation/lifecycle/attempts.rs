use super::super::{context, delivery, process, store, text};
use anyhow::{Result, ensure};
use serde_json::{Value, json};

pub fn prepare(id: &str, recover: bool) -> Result<Value> {
    context::host_only()?;
    store::transaction(|db| {
        let session = store::get(db, id)?;
        ensure!(
            !super::super::terminal(&session),
            "cannot launch a terminal delegation"
        );
        ensure!(
            !process::owns_processes(&session),
            "delegation still has a live process; cancel it before starting another attempt"
        );
        let attempt = session["attempt"].as_u64().unwrap_or_default();
        ensure!(
            attempt == 0 || recover,
            "delegation was already launched; use recover"
        );
        ensure!(
            attempt == 0
                || super::super::now() - session["updated_at"].as_f64().unwrap_or_default() >= 1.0
                || text(&session, "status") == "stalled",
            "delegation launch is still initializing"
        );
        let token = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        store::update(
            db,
            id,
            json!({"status": if recover {"recovering"} else {"starting"},
            "attempt": attempt+1, "attempt_token": token, "supervisor_pid": null, "guest_pid": null,
            "supervisor_identity": null, "guest_identity": null, "guest_heartbeat": null,
            "exit_code": null, "error": null, "updated_at": super::super::now(), "last_activity": super::super::now()}),
        )?;
        store::event(
            db,
            id,
            "system",
            "attempt_started",
            json!({"attempt": attempt+1, "recovery":recover}),
            None,
            None,
        )?;
        let updated = store::get(db, id)?;
        delivery::stage(
            db,
            &updated,
            if recover { "recovering" } else { "starting" },
            "system",
            "",
        )?;
        Ok(updated)
    })
}

pub fn attach(id: &str, pid: u32) -> Result<()> {
    store::transaction(|db| {
        let session = store::get(db, id)?;
        context::authorize(&session, "guest")?;
        ensure!(
            !super::super::terminal(&session),
            "delegation already ended"
        );
        let supervisor = std::process::id();
        store::update(
            db,
            id,
            json!({"status": "running", "supervisor_pid":supervisor,
            "supervisor_identity": process::identity(supervisor), "guest_pid":pid,
            "guest_identity": process::identity(pid), "guest_heartbeat":super::super::now(),
            "updated_at":super::super::now(), "last_activity":super::super::now()}),
        )?;
        delivery::stage(db, &store::get(db, id)?, "started", "guest", "")
    })
}

pub fn finish(id: &str, code: i32, reason: &str, stdout: &str, stderr: &str) -> Result<()> {
    store::transaction(|db| {
        let session = store::get(db, id)?;
        context::authorize(&session, "guest")?;
        let mut changes = json!({"exit_code":code, "supervisor_pid":null, "guest_pid":null,
            "supervisor_identity":null, "guest_identity":null, "updated_at":super::super::now()});
        if !super::super::terminal(&session) {
            changes["status"] = json!("stalled");
            changes["error"] = json!(if reason.is_empty() {
                format!("guest process exited with code {code} without publishing a result")
            } else {
                reason.to_owned()
            });
        }
        store::update(db, id, changes)?;
        store::event(
            db,
            id,
            "system",
            "process_exit",
            json!({"exit_code":code,"reason":reason,"stdout_tail":stdout,"stderr_tail":stderr}),
            None,
            None,
        )?;
        delivery::stage(db, &store::get(db, id)?, "exited", "guest", reason)
    })
}
