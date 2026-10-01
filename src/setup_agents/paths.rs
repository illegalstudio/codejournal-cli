use anyhow::{Context, Result, bail};
use directories::BaseDirs;
use std::path::PathBuf;

pub const NAMES: [&str; 6] = ["codex", "claude", "cursor", "grok", "kimi", "pi"];

pub fn root(agent: &str) -> Result<PathBuf> {
    let home = BaseDirs::new()
        .context("home directory unavailable")?
        .home_dir()
        .to_path_buf();
    let (variable, suffix) = match agent {
        "codex" => (Some("CODEX_HOME"), ".codex"),
        "claude" => (Some("CLAUDE_CONFIG_DIR"), ".claude"),
        "cursor" => (None, ".cursor"),
        "grok" => (None, ".grok"),
        "kimi" => (Some("KIMI_CODE_HOME"), ".kimi-code"),
        "pi" => (Some("PI_CODING_AGENT_DIR"), ".pi/agent"),
        _ => bail!("unsupported agent: {agent}"),
    };
    Ok(variable
        .and_then(std::env::var_os)
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(suffix)))
}

pub fn skill(agent: &str) -> Result<PathBuf> {
    let root = if agent == "grok" {
        BaseDirs::new()
            .context("home directory unavailable")?
            .home_dir()
            .join(".agents")
    } else {
        root(agent)?
    };
    Ok(root.join("skills/code-journal/SKILL.md"))
}

pub fn instructions(agent: &str) -> Result<Option<PathBuf>> {
    Ok(matches!(agent, "grok" | "kimi" | "pi")
        .then(|| root(agent).map(|root| root.join("AGENTS.md")))
        .transpose()?)
}
