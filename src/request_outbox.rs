use crate::{api::Api, outbox};
use anyhow::{Result, bail};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct PendingRequest {
    pub id: String,
    pub server: String,
    pub method: String,
    pub path: String,
    pub body: Option<Value>,
}

fn directory() -> Result<PathBuf> {
    let root = outbox::directory()?.parent().unwrap().join("requests");
    fs::create_dir_all(&root)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
    }
    Ok(root)
}

pub fn enqueue(request: &PendingRequest) -> Result<()> {
    let epoch = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let target = directory()?.join(format!("{epoch:020}-{}.json", request.id));
    let temporary = target.with_extension("tmp");
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    file.write_all(&serde_json::to_vec(request)?)?;
    file.sync_all()?;
    fs::rename(temporary, target)?;
    Ok(())
}

fn entries() -> Result<Vec<(PathBuf, PendingRequest)>> {
    let mut paths = fs::read_dir(directory()?)?
        .filter_map(|entry| entry.ok().map(|item| item.path()))
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths
        .into_iter()
        .map(|path| Ok((path.clone(), serde_json::from_slice(&fs::read(path)?)?)))
        .collect()
}

pub fn pending() -> Result<usize> {
    Ok(entries()?.len())
}

pub fn flush(api: &Api) -> Result<usize> {
    let lock = OpenOptions::new()
        .write(true)
        .create(true)
        .open(directory()?.join(".flush.lock"))?;
    if lock.try_lock_exclusive().is_err() {
        return Ok(0);
    }
    let mut count = 0;
    for (path, request) in entries()? {
        if request.server != api.server() {
            continue;
        }
        api.replay(&request)?;
        fs::remove_file(path)?;
        count += 1;
    }
    if count == 0 && pending()? > 0 {
        bail!("queued writes belong to another server");
    }
    Ok(count)
}

pub fn new(server: &str, method: &str, path: &str, body: Option<Value>) -> PendingRequest {
    PendingRequest {
        id: Uuid::new_v4().to_string(),
        server: server.to_owned(),
        method: method.to_owned(),
        path: path.to_owned(),
        body,
    }
}

#[derive(Debug)]
pub struct QueuedWrite(pub String);

impl std::fmt::Display for QueuedWrite {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "write queued for synchronization: {}", self.0)
    }
}

impl std::error::Error for QueuedWrite {}
