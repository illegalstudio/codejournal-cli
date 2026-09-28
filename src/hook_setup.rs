use crate::hook_args::HooksAction;
use crate::hook_settings;
use crate::hook_setup_format;
use crate::hook_status;
use crate::output;
use anyhow::{Context, Result, bail};
use directories::BaseDirs;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

pub fn run(action: &HooksAction, json_mode: bool) -> Result<()> {
    let (agent, install, dry_run, status) = match action {
        HooksAction::Install { agent, dry_run } => (agent.as_str(), true, *dry_run, false),
        HooksAction::Uninstall { agent, dry_run } => (agent.as_str(), false, *dry_run, false),
        HooksAction::Status { agent } => (agent.as_str(), false, true, true),
    };
    let agents = if agent == "all" {
        vec!["codex", "claude"]
    } else {
        vec![agent]
    };
    let binary = std::env::current_exe()?.canonicalize()?;
    let mut results = Vec::new();
    for target_agent in agents {
        let path = target(target_agent)?;
        if agent == "all" && !path.parent().is_some_and(|parent| parent.exists()) {
            continue;
        }
        let mut settings = match hook_settings::read(&path) {
            Ok(settings) => settings,
            Err(error) if status => {
                results.push(hook_status::invalid(
                    target_agent,
                    &path,
                    &error.to_string(),
                ));
                continue;
            }
            Err(error) => return Err(error),
        };
        if status {
            results.push(hook_status::collect(target_agent, &path, &settings)?);
            continue;
        }
        let before = settings.clone();
        let removed = hook_settings::installed(&before);
        hook_settings::update(&mut settings, &binary, target_agent, install)?;
        let changed = settings != before;
        let backup = if !dry_run && changed {
            let backup = hook_settings::write(&path, &settings)?;
            if !install && target_agent == "codex" && settings == json!({}) {
                fs::remove_file(&path)?;
            }
            backup
        } else {
            None
        };
        results.push(json!({"agent": target_agent, "settings": path, "installed":
            hook_settings::installed(&settings),
            "changed": changed, "dry_run": dry_run, "backup": backup,
            "removed": if install { Vec::new() } else { removed },
            "preview": if dry_run && install { hook_settings::owned_groups(&settings) } else { json!(null) }}));
    }
    if results.is_empty() {
        bail!("no supported agent settings directory found");
    }
    let text = hook_setup_format::render(&results, status, install, dry_run);
    let mut payload = json!({"agents": results});
    if results.len() == 1 {
        for (key, value) in results[0].as_object().context("invalid hook result")? {
            payload[key] = value.clone();
        }
    }
    output::emit(&payload, &text, json_mode)
}

pub(crate) fn target(agent: &str) -> Result<PathBuf> {
    let home = BaseDirs::new()
        .context("home directory unavailable")?
        .home_dir()
        .to_path_buf();
    match agent {
        "codex" => Ok(std::env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".codex"))
            .join("hooks.json")),
        "claude" => Ok(std::env::var_os("CLAUDE_CONFIG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".claude"))
            .join("settings.json")),
        _ => bail!("unsupported agent: {agent}"),
    }
}
