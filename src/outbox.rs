use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

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
    event["id"] = Value::String(id.clone());
    event["origin"] = serde_json::json!(current_origin());
    let epoch = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let target = directory()?.join(format!("{epoch:020}-{id}.json"));
    let temp = target.with_extension("tmp");
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temp)?;
    file.write_all(&serde_json::to_vec(&event)?)?;
    file.sync_all()?;
    fs::rename(temp, target)?;
    Ok(id)
}

pub fn fingerprint(server: &str, tenant: &str, token: &str) -> String {
    let context = serde_json::json!([server.trim_end_matches('/'), tenant, token]);
    format!("{:x}", Sha256::digest(context.to_string().as_bytes()))
}

fn current_origin() -> Option<String> {
    let config = crate::config::Config::load().ok()?;
    let server = std::env::var("CJ_SERVER_URL").unwrap_or(config.server.clone());
    let token = config.token_for(&server).ok()?;
    Some(fingerprint(&server, &config.tenant, &token))
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
        .map(|path| {
            let value = serde_json::from_slice(&fs::read(&path)?)?;
            Ok((path, value))
        })
        .collect()
}

pub fn spawn_flush() -> Result<()> {
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
