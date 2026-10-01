use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn write(path: &Path, body: &str) -> Result<()> {
    let parent = path.parent().context("invalid agent configuration path")?;
    fs::create_dir_all(parent)?;
    if path.exists() {
        fs::copy(
            path,
            path.with_extension(format!("cj-backup-{}.md", uuid::Uuid::new_v4())),
        )?;
    }
    let temporary = path.with_extension(format!("cj-tmp-{}.md", uuid::Uuid::new_v4()));
    let result = (|| {
        fs::write(&temporary, body)?;
        fs::rename(&temporary, path)?;
        Ok(())
    })();
    if temporary.exists() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
