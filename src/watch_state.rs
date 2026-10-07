use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub fn directory() -> Result<PathBuf> {
    let base = ProjectDirs::from("com", "illegalstudio", "codejournal")
        .context("cannot find state directory")?;
    let path = base
        .state_dir()
        .unwrap_or_else(|| base.data_local_dir())
        .join("watches");
    fs::create_dir_all(&path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(path)
}

pub fn pid_path(id: &str) -> Result<PathBuf> {
    uuid::Uuid::parse_str(id).context("invalid watch ID")?;
    Ok(directory()?.join(format!("{id}.pid")))
}

pub fn write_pid(id: &str, pid: u32, scope: &str) -> Result<()> {
    write_json(
        &pid_path(id)?,
        &serde_json::json!({"pid": pid, "scope": scope}),
    )
}

pub fn read_pid(id: &str, scope: &str) -> Result<u32> {
    let value: serde_json::Value = serde_json::from_slice(&fs::read(pid_path(id)?)?)?;
    if let Some(pid) = value.as_u64() {
        return u32::try_from(pid).context("invalid watch PID");
    }
    anyhow::ensure!(
        value["scope"] == scope,
        "watch PID belongs to another local scope"
    );
    u32::try_from(value["pid"].as_u64().context("invalid watch PID")?).context("invalid watch PID")
}

pub fn write_json(path: &std::path::Path, value: &serde_json::Value) -> Result<()> {
    let mut temporary = tempfile::NamedTempFile::new_in(directory()?)?;
    temporary.write_all(&serde_json::to_vec(value)?)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path)?;
    sync_directory()
}

pub fn sync_directory() -> Result<()> {
    #[cfg(unix)]
    fs::File::open(directory()?)?.sync_all()?;
    Ok(())
}

pub fn remove_pid(id: &str) {
    if let Ok(path) = pid_path(id) {
        let _ = fs::remove_file(path);
    }
}
