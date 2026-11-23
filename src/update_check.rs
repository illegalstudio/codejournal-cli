//! Notices a newer cj release without making any command wait: at most once a day a detached
//! process asks for the latest release, and later commands read what it saved.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const INTERVAL_SECONDS: u64 = 24 * 3600;

#[derive(Default, Serialize, Deserialize)]
struct State {
    checked_at: u64,
    latest: Option<String>,
}

fn path() -> Result<PathBuf> {
    Ok(crate::outbox::directory()?
        .parent()
        .context("cannot find state directory")?
        .join("update-check.json"))
}

fn load() -> State {
    path()
        .ok()
        .and_then(|path| fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn save(state: &State) -> Result<()> {
    let path = path()?;
    let mut temporary = tempfile::NamedTempFile::new_in(path.parent().context("no parent")?)?;
    serde_json::to_writer(&mut temporary, state)?;
    temporary.persist(path)?;
    Ok(())
}

/// Development builds stay quiet unless asked; delegated guests leave the check to the host.
fn enabled() -> bool {
    let choice = std::env::var("CODE_JOURNAL_UPDATE_CHECK").unwrap_or_default();
    choice != "off"
        && (choice == "on" || !cfg!(debug_assertions))
        && std::env::var_os("CJ_DELEGATION_ID").is_none()
}

/// The newer release found by the last check, with the command that installs it here.
pub fn available() -> Option<Value> {
    let latest = semver::Version::parse(&load().latest?).ok()?;
    let current = semver::Version::parse(env!("CARGO_PKG_VERSION")).ok()?;
    (enabled() && latest > current).then(|| {
        json!({"current": current.to_string(), "latest": latest.to_string(),
            "command": crate::distribution::update_command()})
    })
}

pub fn line(update: &Value) -> Option<String> {
    Some(format!(
        "Update available: cj {} is released (installed {}). Tell the user; they can update with `{}`.",
        update["latest"].as_str()?,
        update["current"].as_str()?,
        update["command"].as_str()?
    ))
}

/// Starts the background check when the last one is more than 24 hours old.
pub fn refresh_if_due() {
    let state = load();
    let now = crate::request_age::now();
    if !enabled() || now.saturating_sub(state.checked_at) < INTERVAL_SECONDS {
        return;
    }
    // Recording the attempt first keeps a failing check from being repeated by every command.
    let attempt = State {
        checked_at: now,
        latest: state.latest,
    };
    let Ok(binary) = std::env::current_exe() else {
        return;
    };
    if save(&attempt).is_err() {
        return;
    }
    let mut process = Command::new(binary);
    process
        .arg("update-check")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    crate::watches::process::detached(&mut process);
    let _ = process.spawn();
}

/// Remembers the latest release, whoever asked for it.
pub fn record(latest: &semver::Version) -> Result<()> {
    save(&State {
        checked_at: crate::request_age::now(),
        latest: Some(latest.to_string()),
    })
}

/// The detached process: one bounded request, then exit.
pub fn run() -> Result<()> {
    record(&crate::distribution::latest_release()?)
}
