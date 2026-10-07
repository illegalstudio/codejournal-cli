use crate::request_age;
use anyhow::{Context, Result};
use directories::ProjectDirs;
use fs2::FileExt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

pub struct DeliveryState {
    _lock: File,
    path: PathBuf,
    fingerprint: String,
}

impl DeliveryState {
    pub fn lock(server: &str, token: &str, endpoint: &str, body: &Value) -> Result<Self> {
        let identity = json!([
            server.trim_end_matches('/'),
            token,
            endpoint,
            body["host"],
            body["path"]
        ]);
        let key = format!("{:x}", Sha256::digest(identity.to_string().as_bytes()));
        let project = ProjectDirs::from("com", "illegalstudio", "codejournal")
            .context("cannot find activity state directory")?;
        let directory = project
            .state_dir()
            .context("cannot find activity state directory")?
            .join("activity-delivery");
        fs::create_dir_all(&directory)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
        }
        let path = directory.join(format!("{key}.json"));
        let lock = private_file().open(path.with_extension("lock"))?;
        lock.lock_exclusive()?;
        Ok(Self {
            _lock: lock,
            path,
            fingerprint: format!("{:x}", Sha256::digest(body.to_string().as_bytes())),
        })
    }

    pub fn recently_delivered(&self) -> bool {
        let stored = fs::read(&self.path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
        stored.is_some_and(|stored| {
            stored["fingerprint"] == self.fingerprint
                && stored["acknowledged_at"].as_u64().is_some_and(|seconds| {
                    seconds <= request_age::now()
                        && request_age::now().saturating_sub(seconds) < 900
                })
        })
    }

    pub fn acknowledge(&self) -> Result<()> {
        let temporary = self.path.with_extension("tmp");
        let mut file = private_file().truncate(true).open(&temporary)?;
        file.write_all(&serde_json::to_vec(&json!({
            "fingerprint": self.fingerprint, "acknowledged_at": request_age::now()
        }))?)?;
        file.sync_all()?;
        fs::rename(temporary, &self.path)?;
        Ok(())
    }
}

fn private_file() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
}
