use anyhow::Result;
use std::{fs, io::Write, path::PathBuf};

pub const HOST: &str = include_str!("../../skills/code-journal-delegation/SKILL.md");
pub const GUEST: &str = include_str!("../../skills/code-journal-guest/SKILL.md");

pub fn body(guest: bool) -> &'static str {
    if guest { GUEST } else { HOST }
}

pub fn guest_file() -> Result<PathBuf> {
    let directory = super::store::directory()?.join("guest");
    fs::create_dir_all(&directory)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
    }
    let path = directory.join("SKILL.md");
    if fs::read_to_string(&path).ok().as_deref() != Some(GUEST) {
        let mut file = tempfile::NamedTempFile::new_in(directory)?;
        file.write_all(GUEST.as_bytes())?;
        file.as_file().sync_all()?;
        file.persist(&path)?;
    }
    Ok(path)
}
