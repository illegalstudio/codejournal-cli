use super::{files, paths};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs};

pub fn apply(agent: &str, uninstall: bool, dry: bool, status: bool) -> Result<Value> {
    let root = paths::root(agent)?;
    let skills = if agent == "opencode" {
        root.join("skills")
    } else {
        paths::skill(agent)?
            .parent()
            .and_then(|p| p.parent())
            .context("invalid skill root")?
            .to_path_buf()
    };
    let manifest = root.join(".code-journal-delegation.json");
    let mut hashes: BTreeMap<String, String> = match fs::read(&manifest) {
        Ok(bytes) => serde_json::from_slice(&bytes)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
        Err(error) => return Err(error.into()),
    };
    let desired = [
        ("code-journal-delegation", crate::delegation::guide::HOST),
        ("code-journal-guest", crate::delegation::guide::GUEST),
    ];
    let mut installed = true;
    let mut changed = false;
    let mut operations = Vec::new();
    for (name, body) in desired {
        let path = skills.join(name).join("SKILL.md");
        let before = match fs::read_to_string(&path) {
            Ok(body) => Some(body),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        let owned = before
            .as_ref()
            .is_some_and(|s| hashes.get(name) == Some(&digest(s)));
        installed &= before.as_deref() == Some(body);
        if before.is_some() && before.as_deref() != Some(body) && !owned && !uninstall && !status {
            bail!(
                "Delegation skill was customized or is not managed by CJ; preserve it before setup: {}",
                path.display()
            );
        }
        let update = if uninstall {
            owned
        } else {
            before.as_deref() != Some(body)
        };
        changed |= update;
        operations.push((name, body, path, update));
    }
    if !dry && !status {
        for (name, body, path, update) in operations {
            if update {
                if uninstall {
                    fs::remove_file(&path)?;
                    hashes.remove(name);
                } else {
                    files::write(&path, body)?;
                }
            }
            if !uninstall {
                hashes.insert(name.to_owned(), digest(body));
            }
        }
        if hashes.is_empty() {
            if manifest.exists() {
                fs::remove_file(manifest)?;
            }
        } else {
            let content = serde_json::to_string_pretty(&hashes)?;
            if fs::read_to_string(&manifest).ok().as_deref() != Some(&content) {
                files::write(&manifest, &content)?;
            }
        }
    }
    Ok(
        json!({"installed":if status {installed} else {!uninstall},"changed":changed,"directory":skills}),
    )
}

fn digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}
