use super::super::text;
use anyhow::Result;
use serde_json::Value;
use std::{path::Path, process::Command};

pub fn configure(
    command: &mut Command,
    session: &Value,
    binary: &Path,
    managed: bool,
) -> Result<()> {
    command
        .args([
            "exec",
            "--color",
            "never",
            "--ephemeral",
            "--dangerously-bypass-approvals-and-sandbox",
        ])
        .arg("--cd")
        .arg(text(session, "cwd"));
    if managed {
        command
            .arg("-c")
            .arg(format!(
                "mcp_servers.delegation.command={}",
                serde_json::to_string(&binary.to_string_lossy())?
            ))
            .args(["-c", "mcp_servers.delegation.args=[\"delegation\",\"mcp\"]"]);
        command.arg("-c").arg(format!(
            "mcp_servers.delegation.env_vars={}",
            serde_json::to_string(super::mcp_environment::NAMES)?
        ));
    }
    Ok(())
}
