use super::super::text;
use super::Invocation;
use anyhow::Result;
use serde_json::Value;
use std::io::Write;

pub fn build(
    session: &Value,
    prompt: &str,
    dry: bool,
    mut protection: super::sandbox::Protection,
) -> Result<Invocation> {
    let target = text(session, "target");
    let mut command = super::installed::command(super::executable(target), session, dry)?;
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
                command.arg("--settings").arg(
                    super::super::attention::settings(&binary, text(session, "id")).to_string(),
                );
                command.args(["--verbose", "--include-partial-messages"]);
                let path = if dry {
                    std::path::PathBuf::from("<private MCP configuration>")
                } else {
                    let mut file =
                        tempfile::NamedTempFile::new_in(super::super::store::directory()?)?;
                    file.write_all(
                        serde_json::to_string(&super::mcp_environment::config(&binary))?.as_bytes(),
                    )?;
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
            super::codex::configure(&mut command, session, &binary, managed)?;
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
                if managed {
                    "streaming-messages-json"
                } else {
                    "plain"
                },
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
    protection.apply(&mut command, &binary);
    if target == "grok" && managed && !dry {
        config = Some(super::grok_attention::prepare(
            &binary,
            text(session, "id"),
        )?);
    }
    Ok(Invocation {
        command,
        input,
        _config: config,
        _sandbox: protection.scratch,
        sandbox: protection.report,
        marker: protection.marker,
    })
}
