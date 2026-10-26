use crate::{api::Api, outbox};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub(crate) mod delivery;

#[derive(Clone, Serialize, Deserialize)]
pub struct PendingRequest {
    pub id: String,
    pub server: String,
    pub method: String,
    pub path: String,
    pub body: Option<Value>,
    #[serde(default)]
    pub created_at: Option<u64>,
    #[serde(default)]
    pub origin: Option<String>,
    #[serde(default)]
    pub retry_at: Option<u64>,
}

pub(crate) fn directory() -> Result<PathBuf> {
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

pub(crate) fn entries() -> Result<Vec<(PathBuf, PendingRequest)>> {
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
        .filter_map(|path| {
            let bytes = match fs::read(&path) {
                Ok(bytes) => bytes,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
                Err(error) => return Some(Err(error.into())),
            };
            Some((|| {
                let mut request: PendingRequest = serde_json::from_slice(&bytes)?;
                if request.created_at.is_none() {
                    request.created_at = Some(crate::request_age::legacy_created_at(&path)?);
                }
                Ok((path, request))
            })())
        })
        .collect()
}

pub fn pending() -> Result<usize> {
    Ok(entries()?.len())
}

pub fn has_project_init(server: &str, path: &str, slug: &str) -> Result<bool> {
    Ok(entries()?.into_iter().any(|(_, request)| {
        request.server == server
            && request.method == "POST"
            && request.path == path
            && request
                .body
                .as_ref()
                .is_some_and(|body| body["slug"] == slug)
    }))
}

pub fn flush(api: &Api) -> Result<usize> {
    delivery::flush(api, None, usize::MAX)
}

pub fn new(server: &str, method: &str, path: &str, body: Option<Value>) -> PendingRequest {
    PendingRequest {
        id: Uuid::new_v4().to_string(),
        server: server.to_owned(),
        method: method.to_owned(),
        path: path.to_owned(),
        body,
        created_at: Some(crate::request_age::now()),
        origin: None,
        retry_at: None,
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
