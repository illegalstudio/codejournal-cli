use super::Scope;
use anyhow::Result;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::PathBuf,
};

#[derive(Default, Serialize, Deserialize)]
pub struct State {
    pub last_attempt_at: Option<u64>,
    pub last_sync_at: Option<u64>,
    pub next_retry_at: Option<u64>,
    pub failures: u32,
    pub last_error: Option<String>,
    pub blocked: bool,
}

fn directory() -> Result<PathBuf> {
    let directory = crate::outbox::directory()?
        .parent()
        .unwrap()
        .join("request-sync");
    fs::create_dir_all(&directory)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
    }
    Ok(directory)
}

pub fn try_lock(scope: &Scope) -> Result<Option<File>> {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(directory()?.join(format!("{}.lock", scope.origin)))?;
    match file.try_lock_exclusive() {
        Ok(()) => Ok(Some(file)),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub fn load(scope: &Scope) -> Result<State> {
    let path = directory()?.join(format!("{}.json", scope.origin));
    match fs::read(path) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(State::default()),
        Err(error) => Err(error.into()),
    }
}

pub fn save(scope: &Scope, state: &State) -> Result<()> {
    let root = directory()?;
    let mut temporary = tempfile::NamedTempFile::new_in(&root)?;
    temporary.write_all(&serde_json::to_vec(state)?)?;
    temporary.as_file().sync_all()?;
    temporary.persist(root.join(format!("{}.json", scope.origin)))?;
    Ok(())
}

pub fn synchronized(api: &crate::api::Api, tenant: &str) -> Result<()> {
    let scope = Scope::new(api, tenant);
    if let Some(_lock) = try_lock(&scope)? {
        let state = State {
            last_sync_at: Some(crate::request_age::now()),
            ..State::default()
        };
        save(&scope, &state)?;
    }
    Ok(())
}
