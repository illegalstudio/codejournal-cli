use super::{SKILL, files, instructions, paths};
use crate::{codex_rules, hook_settings, hook_setup};
use anyhow::Result;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

pub fn installed(name: &str) -> Result<bool> {
    if name == "opencode" {
        return super::opencode::installed();
    }
    if paths::skill(name)?.exists() {
        return Ok(true);
    }
    if let Some(path) = paths::instructions(name)? {
        return Ok(instructions::installed(&path));
    }
    if matches!(name, "codex" | "claude") {
        return Ok(
            !hook_settings::installed(&hook_settings::read(&hook_setup::target(name)?)?).is_empty(),
        );
    }
    Ok(false)
}

pub fn apply(
    name: &str,
    binary: &Path,
    uninstall: bool,
    dry_run: bool,
    status: bool,
) -> Result<Value> {
    if name == "opencode" {
        return super::opencode::apply(binary, uninstall, dry_run, status);
    }
    let path = paths::skill(name)?;
    let before = match fs::read_to_string(&path) {
        Ok(body) => Some(body),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let changed = if uninstall {
        before.as_deref() == Some(SKILL)
    } else {
        before.as_deref() != Some(SKILL)
    };
    let mut hooks = Vec::new();
    if matches!(name, "codex" | "claude") {
        let settings_path = hook_setup::target(name)?;
        let mut settings = hook_settings::read(&settings_path)?;
        let prior = settings.clone();
        if !status {
            hook_settings::update(&mut settings, binary, name, !uninstall)?;
        }
        if !status && !dry_run && settings != prior {
            hook_settings::write(&settings_path, &settings)?;
        }
        hooks = hook_settings::installed(&settings);
    }
    let mut result = json!({"agent": name, "skill": path, "skill_installed": if status { before.as_deref() == Some(SKILL) } else { !uninstall },
        "hooks_installed": hooks, "changed": changed, "dry_run": dry_run});
    if let Some(instruction_path) = paths::instructions(name)? {
        result["instructions_changed"] =
            json!(!status && instructions::apply(&instruction_path, &path, uninstall, dry_run)?);
        result["instructions_installed"] = json!(if status {
            instructions::installed(&instruction_path)
        } else {
            !uninstall
        });
        result["instructions"] = json!(instruction_path);
    }
    if !status && !dry_run && changed {
        if uninstall {
            fs::remove_file(&path)?;
        } else {
            files::write(&path, SKILL)?;
        }
    }
    if name == "codex" {
        let rules = codex_rules::path(&paths::root(name)?);
        result["rules_changed"] =
            json!(!status && codex_rules::apply(&rules, !uninstall, dry_run)?);
        result["rules_installed"] = json!(if status {
            codex_rules::installed(&rules)
        } else {
            !uninstall
        });
        result["rules"] = json!(rules);
    }
    Ok(result)
}
