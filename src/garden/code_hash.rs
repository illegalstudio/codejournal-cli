use super::directory_files;
use crate::git_paths;
use anyhow::{Context, Result, bail};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path};

/// Snapshot only cited paths. Unrelated commits do not invalidate a completed review.
pub fn record(row: &Value, root: &Path) -> Result<String> {
    let mut paths = row["refs"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|item| item["kind"] == "path")
        .filter_map(|item| item["value"].as_str())
        .filter(|path| git_paths::safe_path(path))
        .map(|path| git_paths::reference_path(path, root))
        .collect::<Vec<_>>();
    paths.sort_unstable();
    paths.dedup();
    let mut hash = Sha256::new();
    for path in paths {
        hash.update(path.as_bytes());
        hash.update([0]);
        let file = root.join(path);
        hash.update(if file.symlink_metadata().is_ok() {
            b"present".as_slice()
        } else {
            b"missing".as_slice()
        });
        if file.symlink_metadata().is_ok_and(|meta| !meta.is_dir()) {
            content(&file, root, &mut hash)?;
        } else {
            directory_files::visit(root, path, |file| {
                let name =
                    std::str::from_utf8(file).context("Garden reference has a non-UTF-8 path")?;
                if git_paths::safe_path(name) && root.join(name).symlink_metadata().is_ok() {
                    hash.update(file);
                    content(&root.join(name), root, &mut hash)?;
                }
                Ok(())
            })
            .with_context(|| format!("Cannot snapshot garden reference {path}"))?;
        }
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn content(path: &Path, root: &Path, hash: &mut Sha256) -> Result<()> {
    let meta = path.symlink_metadata()?;
    if meta.is_symlink() {
        hash.update(fs::read_link(path)?.to_string_lossy().as_bytes());
    } else if meta.is_file() {
        if !path.canonicalize()?.starts_with(root.canonicalize()?) {
            bail!(
                "A garden path reference points outside the checkout; use an explicit file reference for external files"
            );
        }
        let mut file = fs::File::open(path)?;
        let mut chunk = [0u8; 32768];
        loop {
            let read = file.read(&mut chunk)?;
            if read == 0 {
                break;
            }
            hash.update(&chunk[..read]);
        }
    }
    hash.update([0]);
    Ok(())
}
