use crate::{api::Api, config::Config, output};
use anyhow::{Result, bail};
use fs2::FileExt;
use serde_json::{Value, json};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

fn lock() -> Result<File> {
    let lock = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(super::directory()?.join(".flush.lock"))?;
    if lock.try_lock_exclusive().is_err() {
        bail!("hook synchronization is running; retry later");
    }
    Ok(lock)
}

fn find(id: &str) -> Result<(PathBuf, Value)> {
    let prefix = id.replace('-', "").to_lowercase();
    if prefix.len() < 8 || !prefix.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("hook event ID must be at least eight hexadecimal characters");
    }
    let mut matches = super::entries()?.into_iter().filter(|(_, event)| {
        event["id"]
            .as_str()
            .is_some_and(|id| id.replace('-', "").to_lowercase().starts_with(&prefix))
    });
    let item = matches
        .next()
        .ok_or_else(|| anyhow::anyhow!("hook event ID is missing or ambiguous"))?;
    if matches.next().is_some() {
        bail!("hook event ID is missing or ambiguous");
    }
    Ok(item)
}

pub fn bind(id: &str, server: Option<&str>, json_mode: bool) -> Result<()> {
    let config = Config::load()?;
    let server = server.unwrap_or(&config.server);
    let api = Api::new(server, &config.token_for(server)?)?;
    let _lock = lock()?;
    let (path, mut event) = find(id)?;
    if super::origin::bound(&event).is_some() {
        bail!("hook event is already bound; use its original account/server");
    }
    event["origin"] = json!(super::fingerprint(api.server(), &config.tenant, &api.token));
    if let Some(fields) = event.as_object_mut() {
        fields.remove("origin_unavailable_reason");
    }
    let mut temp = tempfile::NamedTempFile::new_in(super::directory()?)?;
    temp.write_all(&serde_json::to_vec(&event)?)?;
    temp.as_file().sync_all()?;
    temp.persist(path).map_err(|failure| failure.error)?;
    output::emit(
        &json!({"bound": event["id"], "server": api.server(), "tenant": config.tenant, "synced": false}),
        &format!(
            "Bound hook event {} to {} / {}. Run cj sync to send it.",
            event["id"].as_str().unwrap_or(""),
            api.server(),
            config.tenant
        ),
        json_mode,
    )
}

pub fn drop(id: &str, json_mode: bool) -> Result<()> {
    let _lock = lock()?;
    let (path, event) = find(id)?;
    fs::remove_file(path)?;
    output::emit(
        &json!({"dropped_hook_event": event["id"]}),
        &format!(
            "Dropped queued hook event {}.",
            event["id"].as_str().unwrap_or("")
        ),
        json_mode,
    )
}
