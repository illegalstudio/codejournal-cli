pub(super) mod adapters;
mod codex;
mod grok_attention;
mod installed;
mod launch;
mod mcp_environment;
mod prompt;
mod sandbox;
mod synchronous;
pub use launch::spawn;
pub use mcp_environment::config as mcp_config;
pub use synchronous::run as synchronous;
pub fn notice(value: &Value) {
    sandbox::notice(value);
}
pub fn isolation_summary(value: &Value) -> String {
    sandbox::summary(value)
}
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::{path::PathBuf, process::Command};

pub struct Invocation {
    pub command: Command,
    pub input: Option<String>,
    pub _config: Option<tempfile::NamedTempFile>,
    pub _sandbox: Option<tempfile::TempDir>,
    pub sandbox: Value,
    pub marker: Option<tempfile::NamedTempFile>,
}

pub fn executable(target: &str) -> &'static str {
    match target {
        "claude" => "claude",
        "codex" => "codex",
        "grok" => "grok",
        "kimi" => "kimi",
        "pi" => "pi",
        "cursor" if find("agent").is_some() => "agent",
        "cursor" => "cursor-agent",
        _ => "",
    }
}

fn find(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?).find_map(|directory| {
        let path = directory.join(name);
        #[cfg(windows)]
        let path = path.with_extension("exe");
        path.is_file().then_some(path)
    })
}

pub fn available(target: &str) -> Result<()> {
    let name = executable(target);
    ensure!(
        !name.is_empty() && find(name).is_some(),
        "target agent CLI is not installed or available in PATH: {target}"
    );
    Ok(())
}

pub fn build(session: &Value, dry: bool) -> Result<Invocation> {
    build_with_protection(session, dry, sandbox::prepare(session, dry))
}

fn build_with_protection(
    session: &Value,
    dry: bool,
    mut protection: sandbox::Protection,
) -> Result<Invocation> {
    protection.redact(session);
    let prompt = prompt::build(session, &protection.report);
    adapters::build(session, &prompt, dry, protection)
}

pub fn dry_run(session: &Value) -> Result<Value> {
    let invocation = build(session, true)?;
    Ok(
        json!({"dry_run":true,"executable":invocation.command.get_program().to_string_lossy(),
        "args":invocation.command.get_args().map(|s|s.to_string_lossy().into_owned()).collect::<Vec<_>>(),
        "stdin":invocation.input,"cwd":session["cwd"],"sandbox":invocation.sandbox}),
    )
}
