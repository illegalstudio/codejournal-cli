use anyhow::{Result, ensure};
use std::{fs, io::Write, path::Path};

/// Grok merges global command hooks without changing the user's existing settings.
pub fn prepare(binary: &Path, id: &str) -> Result<tempfile::NamedTempFile> {
    let root = crate::setup_agents::paths::root("grok")?;
    fs::create_dir_all(&root)?;
    let root = root.canonicalize()?;
    let hooks = root.join("hooks");
    if let Ok(metadata) = hooks.symlink_metadata() {
        ensure!(
            metadata.is_dir() && !metadata.file_type().is_symlink(),
            "Grok hook directory must be a real directory"
        );
    }
    fs::create_dir_all(&hooks)?;
    let mut file = tempfile::Builder::new()
        .prefix("cj-delegation-")
        .suffix(".json")
        .tempfile_in(hooks)?;
    file.write_all(serde_json::to_vec(&super::super::attention::settings(binary, id))?.as_slice())?;
    Ok(file)
}
