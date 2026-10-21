use super::super::{args, lifecycle, process, reads, store, text};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::{
    fs::OpenOptions,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub fn launch(id: &str, recover: bool) -> Result<Value> {
    let session = lifecycle::prepare(id, recover)?;
    let log = store::directory()?.join(format!("{id}-supervisor.log"));
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let output = options.open(log)?;
    let mut command = Command::new(std::env::current_exe()?);
    let captured: Value = serde_json::from_str(text(&session, "journal_context"))?;
    if captured["offline"] == true {
        command.arg("--offline").env("CJ_DELEGATION_OFFLINE", "1");
    }
    command
        .args(["delegation", "_worker", id])
        .env("CJ_DELEGATION_ID", id)
        .env("CJ_DELEGATION_ROLE", "guest")
        .env("CJ_DELEGATION_TOKEN", text(&session, "attempt_token"))
        .env("CJ_DELEGATION_ATTEMPT", session["attempt"].to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(output);
    crate::watches::process::detached(&mut command);
    let child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            store::transaction(|db| {
                store::update(
                    db,
                    id,
                    json!({"status":"stalled","error":format!("cannot launch supervisor: {error}"),"updated_at":super::super::now()}),
                )
            })?;
            return Err(error.into());
        }
    };
    let pid = child.id();
    store::transaction(|db| {
        let current = store::get(db, id)?;
        if current["attempt_token"] == session["attempt_token"]
            && matches!(text(&current, "status"), "starting" | "recovering")
        {
            store::update(
                db,
                id,
                json!({"supervisor_pid":pid,"supervisor_identity":process::identity(pid)}),
            )?;
        }
        Ok(())
    })?;
    std::thread::spawn(move || {
        let mut child = child;
        let _ = child.wait();
    });
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let current = store::get(&store::open()?, id)?;
        if current["guest_pid"].is_number()
            || super::super::terminal(&current)
            || text(&current, "status") == "stalled"
        {
            let read = args::ReadArgs {
                context: args::ContextArgs {
                    id: id.to_owned(),
                    role: None,
                },
                after: None,
                full: false,
            };
            return reads::snapshot(&read, false);
        }
        ensure!(
            Instant::now() < until,
            "delegation {id} startup was not confirmed; inspect cj delegation status {id}"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}
