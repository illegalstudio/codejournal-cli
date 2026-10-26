use super::super::files;
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn digest(body: &str) -> String {
    format!("{:x}", Sha256::digest(body.as_bytes()))
}

pub fn read(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(body) => Ok(Some(body)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub struct Managed {
    root: PathBuf,
    manifest: PathBuf,
    hashes: BTreeMap<String, String>,
}

impl Managed {
    pub fn load(root: &Path, name: &str) -> Result<Self> {
        let manifest = root.join(format!(".code-journal-{name}.json"));
        let hashes = read(&manifest)?
            .map(|body| serde_json::from_str(&body))
            .transpose()
            .with_context(|| format!("invalid ownership manifest: {}", manifest.display()))?
            .unwrap_or_default();
        Ok(Self {
            root: root.to_path_buf(),
            manifest,
            hashes,
        })
    }

    pub fn owns(&self, relative: &str) -> Result<bool> {
        Ok(read(&self.root.join(relative))?
            .is_some_and(|body| self.hashes.get(relative) == Some(&digest(&body))))
    }

    pub fn tracks(&self, relative: &str) -> bool {
        self.hashes.contains_key(relative)
    }

    pub fn apply(
        &self,
        desired: &BTreeMap<String, String>,
        uninstall: bool,
        dry: bool,
        status: bool,
    ) -> Result<Value> {
        let mut modified = Vec::new();
        let mut changes = Vec::new();
        let mut installed = true;
        let mut current = true;
        for (relative, wanted) in desired {
            let path = self.root.join(relative);
            let before = read(&path)?;
            let owned = self.owns(relative)?;
            installed &= before.is_some();
            current &= before.as_ref() == Some(wanted);
            if before.is_some() && !owned && before.as_ref() != Some(wanted) {
                modified.push(path.display().to_string());
            }
            if if uninstall {
                owned
            } else {
                before.as_ref() != Some(wanted)
            } {
                changes.push(relative.clone());
            }
        }
        if !status && !uninstall && !modified.is_empty() {
            bail!(
                "OpenCode files were customized or are not managed by CJ; preserve or move them before setup: {}",
                modified.join(", ")
            );
        }
        // Keep a customized plugin and its helper files together on uninstall.
        if uninstall && !modified.is_empty() {
            changes.clear();
        }
        if !status && !dry {
            let mut hashes = self.hashes.clone();
            for relative in &changes {
                let path = self.root.join(relative);
                if uninstall {
                    fs::remove_file(&path)?;
                    hashes.remove(relative);
                } else {
                    files::write(&path, &desired[relative])?;
                }
            }
            if !uninstall {
                for (relative, body) in desired {
                    hashes.insert(relative.clone(), digest(body));
                }
            }
            if hashes.is_empty() {
                if self.manifest.exists() {
                    fs::remove_file(&self.manifest)?;
                }
            } else {
                let body = serde_json::to_string_pretty(&hashes)?;
                if read(&self.manifest)?.as_ref() != Some(&body) {
                    files::write(&self.manifest, &body)?;
                }
            }
        }
        Ok(
            json!({"installed": installed, "current": current, "modified": modified,
            "changed": !changes.is_empty(), "files": changes}),
        )
    }
}
