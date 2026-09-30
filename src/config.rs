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
        if crate::keyring_store::write(&config.key(), &token).is_err() {
            config.fallback_token = Some(token);
            eprintln!("System keyring unavailable. Token stored in a private config file.");
        }
        config.save()
    }

    pub fn token(&self) -> Result<String> {
        if let Ok(token) = std::env::var("CJ_TOKEN") {
            return Ok(token);
        }
        let stored = crate::keyring_store::read(&self.key());
        match (stored, &self.fallback_token) {
            (Ok(token), _) => Ok(token),
            (Err(_), Some(token)) => Ok(token.clone()),
            (Err(keyring::Error::NoEntry), None) => bail!("token missing; run cj login"),
            (Err(error), None) => bail!(
                "cannot read the Code Journal token from the system keyring ({error}); \
                 if this runs inside an agent sandbox, run cj outside it \
                 (the user can allow that for Codex with `cj setup agents`)"
            ),
        }
    }

    /// Returns the stored token only for the server that issued it.
    pub fn token_for(&self, server: &str) -> Result<String> {
        if server.trim_end_matches('/') != self.server.trim_end_matches('/')
            && std::env::var_os("CJ_TOKEN").is_none()
        {
            bail!(
                "the stored token belongs to {}; set CJ_TOKEN to use {server}",
                self.server
            );
        }
        self.token()
    }

    pub fn logout(&mut self) -> Result<()> {
        let _ = crate::keyring_store::delete(&self.key());
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
