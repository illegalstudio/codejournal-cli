use super::super::{contention, context, process, store, text, workspace};
use anyhow::Result;
use serde_json::{Value, json};
use std::{
    process::Child,
    time::{Duration, Instant},
};

pub fn run(id: &str, child: &mut Child) -> Result<String> {
    let started = Instant::now();
    let mut reason = String::new();
    let mut warned = None;
    while child.try_wait()?.is_none() {
        if crate::watches::process::stopped() {
            reason = "supervisor was interrupted".to_owned();
            process::stop(child);
            break;
        }
        match tick(id, started) {
            Ok(Some(value)) => {
                reason = value;
                process::stop(child);
                break;
            }
            Ok(None) => {}
            Err(error) if contention::is_busy(&error) => {
                if warned.is_none_or(|at: Instant| at.elapsed() >= Duration::from_secs(30)) {
                    eprintln!(
                        "Delegation database is busy; preserving guest and retrying: {error:#}"
                    );
                    warned = Some(Instant::now());
                }
                std::thread::sleep(Duration::from_millis(200));
                continue;
            }
            Err(error) => return Err(error),
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    Ok(reason)
}

fn tick(id: &str, started: Instant) -> Result<Option<String>> {
    let current = store::get(&store::open()?, id)?;
    context::authorize(&current, "guest")?;
    if super::super::terminal(&current) {
        return Ok(Some(String::new()));
    }
    let reason = check(id, &current, started)?;
    if !reason.is_empty() {
        return Ok(Some(reason));
    }
    if super::super::now() - current["guest_heartbeat"].as_f64().unwrap_or_default() < 1.0 {
        return Ok(None);
    }
    store::transaction(|db| {
        let session = store::get(db, id)?;
        context::authorize(&session, "guest")?;
        store::update(db, id, json!({"guest_heartbeat":super::super::now()}))
    })?;
    Ok(None)
}

fn check(id: &str, current: &Value, started: Instant) -> Result<String> {
    if text(current, "mode") == "write" {
        let status = match workspace::changed(current, true) {
            Ok(status) => status,
            Err(error) => return Ok(error.to_string()),
        };
        if current["diff_detected_at"].is_null() {
            if !status.is_empty() {
                store::transaction(|db| {
                    let fresh = store::get(db, id)?;
                    context::authorize(&fresh, "guest")?;
                    store::update(
                        db,
                        id,
                        json!({"diff_detected_at":super::super::now(),"last_activity":super::super::now()}),
                    )?;
                    store::event(
                        db,
                        id,
                        "system",
                        "workspace_changed",
                        json!({"status":status.chars().take(65536).collect::<String>()}),
                        None,
                        Some("first-workspace-change"),
                    )?;
                    Ok(())
                })?;
            } else if started.elapsed().as_secs_f64()
                >= current["diff_timeout"].as_f64().unwrap_or(300.0)
                && text(current, "status") != "waiting_for_host"
            {
                return Ok(
                    "write guest produced no workspace diff within its diff timeout".to_owned(),
                );
            }
        }
    }
    let timeout = current["stall_timeout"].as_f64().unwrap_or(600.0);
    let last = current["last_activity"].as_f64().unwrap_or_default();
    if timeout > 0.0
        && super::super::now() - last >= timeout
        && text(current, "status") != "waiting_for_host"
    {
        return Ok(format!("guest produced no activity for {timeout} seconds"));
    }
    Ok(String::new())
}
