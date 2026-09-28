use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

pub fn directory() -> Result<PathBuf> {
    let base = ProjectDirs::from("com", "illegalstudio", "codejournal")
        .context("cannot find state directory")?;
    let path = base
        .state_dir()
        .context("cannot find state directory")?
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
    if !id
        .chars()
        .all(|letter| letter.is_ascii_hexdigit() || letter == '-')
    {
        anyhow::bail!("invalid watch ID");
    }
    Ok(directory()?.join(format!("{id}.pid")))
}

pub fn write_pid(id: &str, pid: u32) -> Result<()> {
    let path = pid_path(id)?;
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    write!(file, "{pid}")?;
    Ok(())
}

pub fn read_pid(id: &str) -> Result<u32> {
    Ok(fs::read_to_string(pid_path(id)?)?.parse()?)
}

pub fn remove_pid(id: &str) {
    if let Ok(path) = pid_path(id) {
        let _ = fs::remove_file(path);
    }
}
