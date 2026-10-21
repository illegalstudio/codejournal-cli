use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub(crate) mod commands;
pub(crate) mod diagnostics;
mod inventory;
mod mutation;
pub(crate) mod origin;

pub fn directory() -> Result<PathBuf> {
    let root = ProjectDirs::from("com", "illegalstudio", "codejournal")
        .context("cannot find state directory")?;
    let directory = root
        .state_dir()
        .context("cannot find state directory")?
        .join("outbox");
    fs::create_dir_all(&directory)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
    }
    Ok(directory)
}

pub fn enqueue(mut event: Value) -> Result<String> {
    let id = Uuid::new_v4().to_string();
    origin::capture(&mut event);
    let epoch = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    enqueue_captured(event, &id, epoch)?;
    Ok(id)
}

pub(crate) fn enqueue_captured(mut event: Value, id: &str, epoch: u128) -> Result<()> {
    Uuid::parse_str(id)?;
    event["id"] = Value::String(id.to_owned());
    let target = directory()?.join(format!("{epoch:020}-{id}.json"));
    let mut file = tempfile::NamedTempFile::new_in(directory()?)?;
    file.write_all(&serde_json::to_vec(&event)?)?;
    file.as_file().sync_all()?;
    if let Err(error) = file.persist_noclobber(&target) {
        if error.error.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(error.error.into());
        }
        let existing: Value = serde_json::from_slice(&fs::read(&target)?)?;
        anyhow::ensure!(
            existing == event,
            "outbox event identity already contains different data"
        );
    }
    Ok(())
}

pub fn fingerprint(server: &str, tenant: &str, token: &str) -> String {
    let context = serde_json::json!([server.trim_end_matches('/'), tenant, token]);
    format!("{:x}", Sha256::digest(context.to_string().as_bytes()))
}

pub fn pending() -> Result<usize> {
    Ok(entries()?.len())
}

pub fn entries() -> Result<Vec<(PathBuf, Value)>> {
    let mut paths = fs::read_dir(directory()?)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|value| value == "json"))
        .collect::<Vec<_>>();
    paths.sort();
    paths
        .into_iter()
        .filter_map(|path| match fs::read(&path) {
            Ok(bytes) => Some(
                serde_json::from_slice(&bytes)
                    .map(|value| (path, value))
                    .map_err(Into::into),
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => Some(Err(error.into())),
        })
        .collect()
}

pub fn spawn_flush() -> Result<()> {
    let _ = crate::request_sync::wake_configured();
    if std::env::var("CODE_JOURNAL_HOOK_FLUSH").as_deref() == Ok("off") {
        return Ok(());
    }
    let mut process = Command::new(std::env::current_exe()?);
    process
        .arg("hook-flush")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if std::env::var("CODE_JOURNAL_HOOK_SYNC").as_deref() == Ok("1") {
        process.status()?;
    } else {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            process.process_group(0);
        }
        process.spawn()?;
    }
    Ok(())
}
