mod files;
mod install;
mod instructions;
mod paths;

use crate::output;
use anyhow::{Result, bail};
use clap::{Args, Subcommand};
use serde_json::json;

const SKILL: &str = include_str!("../../skill/SKILL.md");

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
        #[arg(long, default_value = "all", value_parser = ["all", "codex", "claude", "cursor", "grok", "kimi", "pi"])]
        agent: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, conflicts_with_all = ["status", "refresh"])]
        uninstall: bool,
        #[arg(long, conflicts_with = "refresh")]
        status: bool,
        /// Refresh only agents that already have Code Journal installed.
        #[arg(long)]
        refresh: bool,
    },
}

pub fn run(action: &SetupAction) -> Result<()> {
    output::json(&collect(action)?)
}

pub fn refresh() -> Result<()> {
    collect(&SetupAction::Agents {
        agent: "all".to_owned(),
        dry_run: false,
        uninstall: false,
        status: false,
        refresh: true,
    })?;
    Ok(())
}

fn collect(action: &SetupAction) -> Result<serde_json::Value> {
    let SetupAction::Agents {
        agent,
        dry_run,
        uninstall,
        status,
        refresh,
    } = action;
    let names = if agent == "all" {
        paths::NAMES.to_vec()
    } else {
        vec![agent.as_str()]
    };
    let binary = std::env::current_exe()?.canonicalize()?;
    let mut results = Vec::new();
    for name in names {
        if agent == "all" && !paths::root(name)?.exists() {
            continue;
        }
        if *refresh && !install::installed(name)? {
            continue;
        }
        results.push(install::apply(
            name, &binary, *uninstall, *dry_run, *status,
        )?);
    }
    if results.is_empty() && !refresh {
        bail!("no supported agent directory found");
    }
    Ok(json!({"agents": results}))
}
