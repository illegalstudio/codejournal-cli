use anyhow::{Context, Result, bail};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

#[derive(Default, Deserialize, Serialize)]
pub struct Config {
    pub server: String,
    pub tenant: String,
    #[serde(default)]
    pub notifications: NotificationSettings,
    fallback_token: Option<String>,
}

#[derive(Default, Deserialize, Serialize)]
pub struct NotificationSettings {
    #[serde(default)]
    pub desktop: bool,
    pub ntfy_url: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = path()?;
        let bytes = fs::read(&path)
            .with_context(|| format!("not logged in; run cj login ({})", path.display()))?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub fn store(server: String, tenant: String, token: String) -> Result<()> {
        let notifications = Self::load()
            .ok()
            .filter(|current| current.server == server && current.tenant == tenant)
            .map(|current| current.notifications)
            .unwrap_or_default();
        let mut config = Self {
            server,
            tenant,
            notifications,
            fallback_token: None,
        };
        if keyring::Entry::new("codejournal", &config.key())
            .and_then(|entry| entry.set_password(&token))
            .is_err()
        {
            config.fallback_token = Some(token);
            eprintln!("System keyring unavailable. Token stored in a private config file.");
        }
        config.save()
    }

    pub fn token(&self) -> Result<String> {
        if let Ok(token) = std::env::var("CJ_TOKEN") {
            return Ok(token);
        }
        if let Ok(token) =
            keyring::Entry::new("codejournal", &self.key()).and_then(|entry| entry.get_password())
        {
            return Ok(token);
        }
        self.fallback_token
            .clone()
            .context("token missing; run cj login")
    }

    pub fn logout(&mut self) -> Result<()> {
        if let Ok(entry) = keyring::Entry::new("codejournal", &self.key()) {
            let _ = entry.delete_credential();
        }
        self.fallback_token = None;
        let path = path()?;
        if path.exists() {
            fs::remove_file(path)?;
        }
        println!("Logged out of Code Journal.");
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let path = path()?;
        let parent = path.parent().context("invalid config directory")?;
        fs::create_dir_all(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
        }
        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(0o600))?;
        }
        file.write_all(serde_json::to_string_pretty(self)?.as_bytes())?;
        Ok(())
    }

    fn key(&self) -> String {
        format!("{}:{}", self.server, self.tenant)
    }
}

pub fn path() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("com", "illegalstudio", "codejournal")
        .context("cannot find user config directory")?;
    let path = dirs.config_dir().join("config.json");
    if path.file_name().is_none() {
        bail!("invalid config path");
    }
    Ok(path)
}
