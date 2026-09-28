use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
pub struct SessionState {
    pub repo_common: Option<String>,
    pub commits: Vec<String>,
    pub logged: Vec<String>,
    pub agent: String,
    pub session_id: String,
    pub cwd: String,
    pub root: Option<String>,
    pub started_at: String,
    pub turn_started_at: String,
    pub turn_head: Option<String>,
    pub turn_files: Vec<String>,
    pub files: HashMap<String, String>,
    pub last_log_at: Option<String>,
    pub reminded_turn: Option<String>,
    pub waiting: bool,
    pub notified_at: Option<String>,
    pub delegation_id: Option<String>,
    pub ended_at: Option<String>,
    pub last_event_at: Option<String>,
}

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn current_id() -> Option<String> {
    [
        "CJ_SESSION_ID",
        "CLAUDE_CODE_SESSION_ID",
        "CODEX_THREAD_ID",
        "CURSOR_SESSION_ID",
    ]
    .into_iter()
    .find_map(|name| std::env::var(name).ok().filter(|id| !id.is_empty()))
}

fn directory() -> Result<PathBuf> {
    let base = ProjectDirs::from("com", "illegalstudio", "codejournal")
        .context("cannot find state directory")?;
    let directory = base
        .state_dir()
        .context("cannot find state directory")?
        .join("sessions");
    fs::create_dir_all(&directory)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
    }
    Ok(directory)
}

fn path(session: &str) -> Result<PathBuf> {
    let key = format!("{}:{session}", crate::attribution::agent(None));
    let digest = Sha256::digest(key.as_bytes());
    Ok(directory()?.join(format!("{digest:x}.json")))
}

pub fn others() -> Result<Vec<SessionState>> {
    let mut sessions = Vec::new();
    for entry in fs::read_dir(directory()?)? {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            if let Ok(bytes) = fs::read(path) {
                if let Ok(state) = serde_json::from_slice(&bytes) {
                    sessions.push(state);
                }
            }
        }
    }
    Ok(sessions)
}

pub fn load(session: &str) -> Result<SessionState> {
    match fs::read(path(session)?) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(SessionState::default()),
        Err(error) => Err(error.into()),
    }
}

pub fn save(session: &str, state: &SessionState) -> Result<()> {
    let target = path(session)?;
    let temporary = target.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    file.write_all(&serde_json::to_vec(state)?)?;
    file.sync_all()?;
    fs::rename(temporary, target)?;
    Ok(())
}
