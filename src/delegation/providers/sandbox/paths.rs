use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn writable(session: &Value, scratch: &Path, dry: bool) -> Result<Vec<PathBuf>> {
    if dry {
        return Ok(vec![scratch.to_owned()]);
    }
    for name in ["tmp", "cache", "runtime"] {
        fs::create_dir_all(scratch.join(name))?;
    }
    let state = super::super::super::store::directory()?
        .parent()
        .context("invalid state directory")?
        .canonicalize()?;
    let mut paths = vec![state, scratch.canonicalize()?];
    let path = crate::setup_agents::paths::root(super::super::super::text(session, "target"))?;
    paths.push(resolve(&path)?);
    let cwd = super::super::super::text(session, "cwd");
    if let Ok(root) = super::super::super::workspace::git(cwd, &["rev-parse", "--show-toplevel"]) {
        let mut protected = vec![PathBuf::from(root).canonicalize()?];
        if let Ok(common) = super::super::super::workspace::git(
            cwd,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        ) {
            protected.push(PathBuf::from(common).canonicalize()?);
        }
        for path in &paths {
            ensure!(
                protected
                    .iter()
                    .all(|root| !root.starts_with(path) && !path.starts_with(root)),
                "read-only sandbox state overlaps the Git checkout: {}; move provider and CJ state outside the checkout",
                path.display()
            );
        }
    }
    fs::create_dir_all(&path)?;
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn resolve(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return Ok(path.canonicalize()?);
    }
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    };
    let parent = absolute.parent().context("invalid provider state path")?;
    Ok(resolve(parent)?.join(
        absolute
            .file_name()
            .context("invalid provider state path")?,
    ))
}
