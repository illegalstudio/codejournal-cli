use crate::outbox;
use anyhow::Result;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use uuid::Uuid;

fn path(server: &str, token: &str, endpoint: &str) -> Result<PathBuf> {
    let root = outbox::directory()?.parent().unwrap().join("api-cache");
    fs::create_dir_all(&root)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
    }
    let key = Sha256::digest(format!("{server}\0{token}\0{endpoint}").as_bytes());
    Ok(root.join(format!("{key:x}.json")))
}

pub fn read(server: &str, token: &str, endpoint: &str) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(path(
        server, token, endpoint,
    )?)?)?)
}

pub fn write(server: &str, token: &str, endpoint: &str, value: &Value) -> Result<()> {
    let target = path(server, token, endpoint)?;
    let temporary = target.with_extension(format!("{}.tmp", Uuid::new_v4()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    file.write_all(&serde_json::to_vec(value)?)?;
    file.sync_all()?;
    fs::rename(temporary, target)?;
    Ok(())
}
