use crate::git;
use anyhow::{Context, Result, bail};
use std::path::{Component, Path, PathBuf};

pub fn normalize(raw: &str) -> Result<String> {
    normalize_with_root(raw, git::root().as_deref())
}

fn normalize_with_root(raw: &str, root: Option<&Path>) -> Result<String> {
    let expanded = if raw.starts_with('~') {
        directories::BaseDirs::new()
            .context("home directory unavailable")?
            .home_dir()
            .join(raw.trim_start_matches('~').trim_start_matches('/'))
    } else {
        PathBuf::from(raw)
    };
    if expanded
        .components()
        .any(|part| matches!(part, Component::ParentDir))
    {
        bail!("path ref cannot leave the repository: {raw}");
    }
    let first_dir = expanded
        .components()
        .nth(1)
        .map(|part| Path::new("/").join(part.as_os_str()));
    let physical = expanded.is_absolute()
        && (raw.starts_with('~') || first_dir.is_some_and(|part| part.exists()));
    if physical {
        let root = root.context("path ref is not tied to a repository checkout")?;
        let relative = expanded.strip_prefix(root).map_err(|_| {
            anyhow::anyhow!("path ref is outside this repository; use url: for another repository")
        })?;
        if expanded.exists() && !expanded.canonicalize()?.starts_with(root.canonicalize()?) {
            bail!("path ref is outside this repository; use url: for another repository");
        }
        return Ok(relative.to_string_lossy().to_string());
    }
    Ok(raw.trim_matches('/').to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_external_path_is_rejected() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(normalize_with_root("/tmp/other-project/file.rs", Some(root)).is_err());
        assert_eq!(normalize_with_root("/docs/", Some(root)).unwrap(), "docs");
        let own = root.join("Cargo.toml");
        assert_eq!(
            normalize_with_root(own.to_str().unwrap(), Some(root)).unwrap(),
            "Cargo.toml"
        );
    }
}
