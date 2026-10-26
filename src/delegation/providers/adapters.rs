use super::super::text;
use super::Invocation;
use anyhow::Result;
use serde_json::{Value, json};
use std::{io::Write, process::Command};

pub fn build(session: &Value, prompt: &str, dry: bool) -> Result<Invocation> {
    let target = text(session, "target");
    let mut command = Command::new(super::executable(target));
    let mut config = None;
    let mut input = None;
    let managed = session["id"].is_string();
    let binary = std::env::current_exe()?;
    match target {
        "claude" => {
            command.args([
                "--print",
                "--output-format",
                if managed { "stream-json" } else { "text" },
                "--no-session-persistence",
                "--permission-prompts",
                "none",
                "--dangerously-skip-permissions",
            ]);
            if managed {
                command.args(["--verbose", "--include-partial-messages"]);
                let path = if dry {
                    std::path::PathBuf::from("<private MCP configuration>")
                } else {
                    let mut file =
                        tempfile::NamedTempFile::new_in(super::super::store::directory()?)?;
                    file.write_all(serde_json::to_string(&mcp_config(&binary))?.as_bytes())?;
                    let path = file.path().to_path_buf();
                    config = Some(file);
                    path
                };
                command
                    .arg("--mcp-config")
                    .arg(path)
                    .arg("--strict-mcp-config");
            }
        }
        "codex" => {
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
            input = Some(prompt.to_owned());
        }
        "cursor" => {
            command
                .args([
                    "--print",
                    "--output-format",
                    "text",
                    "--trust",
                    "--yolo",
                    "--sandbox",
                    "disabled",
                ])
                .arg("--workspace")
                .arg(text(session, "cwd"));
        }
        "grok" => {
            command.arg("--cwd").arg(text(session, "cwd")).args([
                "--output-format",
                if managed { "streaming-json" } else { "plain" },
                "--always-approve",
            ]);
            if managed {
                command.arg("--include-partial-messages");
            }
        }
        "kimi" => {
            command.args(["--output-format", "text"]);
        }
        "pi" => {
            command.args(["--print", "--no-session", "--approve"]);
            if managed {
                let path = if dry {
                    std::path::PathBuf::from("<embedded guest skill>")
                } else {
                    super::super::guide::guest_file()?
                };
                command.arg("--skill").arg(path);
            }
        }
        _ => anyhow::bail!("unsupported target: {target}"),
    }
    if !text(session, "model").is_empty() {
        command.arg("--model").arg(text(session, "model"));
    }
    match target {
        "codex" => {
            command.arg("-");
        }
        "grok" => {
            command.arg("--single").arg(prompt);
        }
        "kimi" => {
            command.arg("--prompt").arg(prompt);
        }
        "pi" => {
            command.arg("--").arg(prompt);
        }
        _ => {
            command.arg(prompt);
        }
    }
    command.current_dir(text(session, "cwd"));
    Ok(Invocation {
        command,
        input,
        _config: config,
    })
}

pub fn mcp_config(binary: &std::path::Path) -> Value {
    json!({"mcpServers":{"delegation":{"command":binary,"args":["delegation","mcp"],"env":super::mcp_environment::values()}}})
}
