use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

pub fn write(path: &Path, body: &str) -> Result<()> {
    let parent = path.parent().context("invalid agent configuration path")?;
    fs::create_dir_all(parent)?;
    let permissions = match fs::metadata(path) {
        Ok(metadata) => Some(metadata.permissions()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    if path.exists() {
        let backup = NamedTempFile::new_in(parent)?;
        fs::copy(path, backup.path())?;
        backup.persist(path.with_extension(format!("cj-backup-{}.md", uuid::Uuid::new_v4())))?;
    }
    let mut temporary = NamedTempFile::new_in(parent)?;
    if let Some(permissions) = permissions {
        temporary.as_file().set_permissions(permissions)?;
    }
    temporary.write_all(body.as_bytes())?;
    temporary.as_file().sync_all()?;
    temporary.persist(path)?;
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::write;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn private_replacements_and_backups_keep_the_original_permissions() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("AGENTS.md");
        for mode in [0o600, 0o640, 0o400] {
            fs::write(&path, "private instructions").unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
            write(&path, "preserved instructions and integration").unwrap();
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                mode
            );
            assert_eq!(
                fs::read_to_string(&path).unwrap(),
                "preserved instructions and integration"
            );
        }
        for entry in fs::read_dir(directory.path()).unwrap() {
            let path = entry.unwrap().path();
            assert_eq!(fs::metadata(path).unwrap().permissions().mode() & 0o007, 0);
        }
    }

    #[test]
    fn new_agent_files_are_private() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("SKILL.md");
        write(&path, "new instructions").unwrap();
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
