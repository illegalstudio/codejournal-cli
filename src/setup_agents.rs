use crate::{hook_settings, hook_setup, output};
use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use directories::BaseDirs;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const SKILL: &str = include_str!("../skill/SKILL.md");

#[derive(Args)]
pub struct SetupArgs {
    #[command(subcommand)]
    pub action: Option<SetupAction>,
    #[arg(long)]
    pub database: Option<String>,
    #[arg(long)]
    pub sync_url: Option<String>,
    #[arg(long)]
    pub auth_token: Option<String>,
}

#[derive(Subcommand)]
pub enum SetupAction {
    Agents {
        #[arg(long, default_value = "all", value_parser = ["all", "codex", "claude", "cursor"])]
        agent: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, conflicts_with = "status")]
        uninstall: bool,
        #[arg(long)]
        status: bool,
    },
}

pub fn run(action: &SetupAction) -> Result<()> {
    let SetupAction::Agents {
        agent,
        dry_run,
        uninstall,
        status,
    } = action;
    let agents = if agent == "all" {
        vec!["codex", "claude", "cursor"]
    } else {
        vec![agent.as_str()]
    };
    let binary = std::env::current_exe()?.canonicalize()?;
    let mut results = Vec::new();
    for name in agents {
        let path = skill_path(name)?;
        let root = path
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .context("invalid skill path")?;
        if agent == "all" && !root.exists() {
            continue;
        }
        let before = fs::read_to_string(&path).ok();
        let mut hooks_installed = Vec::new();
        if name != "cursor" {
            let settings_path = hook_setup::target(name)?;
            let mut settings = hook_settings::read(&settings_path)?;
            let prior = settings.clone();
            if !status {
                hook_settings::update(&mut settings, &binary, name, !uninstall)?;
            }
            if !status && !dry_run && settings != prior {
                hook_settings::write(&settings_path, &settings)?;
            }
            hooks_installed = hook_settings::installed(&settings);
        }
        let changed = if *uninstall {
            before.as_deref() == Some(SKILL)
        } else {
            before.as_deref() != Some(SKILL)
        };
        if !status && !dry_run && changed {
            if *uninstall {
                fs::remove_file(&path)?;
            } else {
                write_skill(&path, before.is_some())?;
            }
        }
        results.push(json!({"agent": name, "skill": path, "skill_installed": if *status { before.as_deref() == Some(SKILL) } else { !uninstall },
            "hooks_installed": hooks_installed, "changed": changed, "dry_run": dry_run}));
    }
    if results.is_empty() {
        bail!("no supported agent directory found");
    }
    output::json(&json!({"agents": results}))
}

fn skill_path(agent: &str) -> Result<PathBuf> {
    let home = BaseDirs::new()
        .context("home directory unavailable")?
        .home_dir()
        .to_path_buf();
    let root = match agent {
        "codex" => std::env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".codex")),
        "claude" => std::env::var_os("CLAUDE_CONFIG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".claude")),
        "cursor" => home.join(".cursor"),
        _ => bail!("unsupported agent: {agent}"),
    };
    Ok(root.join("skills/code-journal/SKILL.md"))
}

fn write_skill(path: &Path, had_file: bool) -> Result<()> {
    let parent = path.parent().context("invalid skill path")?;
    fs::create_dir_all(parent)?;
    if had_file {
        let epoch = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
        fs::copy(path, path.with_extension(format!("cj-backup-{epoch}.md")))?;
    }
    let temporary = path.with_extension(format!("cj-tmp-{}.md", uuid::Uuid::new_v4()));
    fs::write(&temporary, SKILL)?;
    fs::rename(temporary, path)?;
    Ok(())
}
