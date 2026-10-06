mod discovery;
mod managed;
pub(crate) mod plugin;

use super::SKILL;
use anyhow::{Result, bail};
pub use discovery::root;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

const SKILL_FILE: &str = "skills/code-journal/SKILL.md";

pub fn installed() -> Result<bool> {
    Ok(root()?.join(".code-journal-skill.json").exists()
        || root()?.join(".code-journal-plugin.json").exists())
}

pub fn apply(binary: &Path, uninstall: bool, dry: bool, status: bool) -> Result<Value> {
    let root = root()?;
    let owned = managed::Managed::load(&root, "skill")?;
    let native = root.join(SKILL_FILE);
    let mut candidates = discovery::skills()?;
    let other_exists = candidates
        .iter()
        .any(|path| !discovery::same_path(path, &native));
    let retire = other_exists && owned.owns(SKILL_FILE)?;
    if retire {
        candidates.retain(|path| !discovery::same_path(path, &native));
    }
    let hashes = candidates
        .iter()
        .map(|path| Ok(managed::digest(&managed::read(path)?.unwrap_or_default())))
        .collect::<Result<HashSet<_>>>()?;
    if hashes.len() > 1 && !uninstall && !status {
        bail!(
            "Conflicting code-journal skills visible to OpenCode. Keep one copy before setup: {}",
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let selected = candidates.first().unwrap_or(&native);
    let invalid_native = candidates.is_empty() && native.exists() && !owned.tracks(SKILL_FILE);
    if invalid_native && !uninstall && !status {
        bail!(
            "Existing OpenCode skill is not a valid code-journal skill; preserve or move it before setup: {}",
            native.display()
        );
    }
    let reuse =
        !discovery::same_path(selected, &native) || (native.exists() && !owned.tracks(SKILL_FILE));
    let desired = BTreeMap::from([(SKILL_FILE.to_owned(), SKILL.to_owned())]);
    // Preflight every managed file before making any change.
    if !reuse || retire || uninstall {
        owned.apply(&desired, uninstall || retire, true, status)?;
    }
    plugin::apply(binary, uninstall, true, status)?;
    let skill = if !reuse || retire || uninstall {
        owned.apply(&desired, uninstall || retire, dry, status)?
    } else {
        json!({"changed": false})
    };
    let hooks = plugin::apply(binary, uninstall, dry, status)?;
    let present = (selected.is_file() && !invalid_native) || (!uninstall && !status);
    Ok(
        json!({"agent": "opencode", "skill": selected, "skill_installed": present,
        "skill_current": managed::read(selected)?.as_deref() == Some(SKILL),
        "skill_reused": reuse, "skill_candidates": candidates, "skill_conflict": hashes.len() > 1,
        "skill_invalid": invalid_native,
        "skill_modified": skill["modified"], "hooks_installed": hooks["installed"],
        "plugin": hooks, "dry_run": dry, "changed": skill["changed"] == true || hooks["changed"] == true}),
    )
}
