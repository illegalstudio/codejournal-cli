use crate::{session_git, session_state};
use anyhow::{Context, Result};
use serde_json::Value;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub fn run(event: &str) -> Result<()> {
    if std::env::var("CODE_JOURNAL_HOOKS").as_deref() == Ok("off") {
        return Ok(());
    }
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let payload: Value = serde_json::from_str(&input).context("hook expects JSON on stdin")?;
    let session = payload["session_id"]
        .as_str()
        .or_else(|| payload["conversation_id"].as_str())
        .map(str::to_owned)
        .or_else(session_state::current_id);
    let cwd = payload["cwd"]
        .as_str()
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?);
    if let Some(session) = session {
        let mut state = session_state::load(&session)?;
        if state.repo_common.is_none() {
            state.repo_common = session_git::common_dir(&cwd);
        }
        if event == "PostToolUse" {
            record_commit(&payload, &cwd, &mut state);
        }
        session_state::save(&session, &state)?;
    }
    if matches!(event, "SessionStart" | "PostToolUse") {
        publish_activity(&cwd)?;
    }
    Ok(())
}

fn record_commit(payload: &Value, cwd: &std::path::Path, state: &mut session_state::SessionState) {
    let command = payload["tool_input"]["command"]
        .as_str()
        .or_else(|| payload["tool_input"]["cmd"].as_str());
    let Some(target) = command.and_then(|text| session_git::commit_target(text, cwd)) else {
        return;
    };
    if session_git::common_dir(&target) != state.repo_common {
        return;
    }
    let response = &payload["tool_response"];
    let output = response["stdout"]
        .as_str()
        .or_else(|| response.as_str())
        .or_else(|| payload["tool_output"].as_str())
        .unwrap_or("");
    let success = response["exit_code"]
        .as_i64()
        .or_else(|| response["exitCode"].as_i64())
        .or_else(|| response["code"].as_i64())
        == Some(0);
    let commit = session_git::commit_from_output(&target, output).or_else(|| {
        if success {
            session_git::head(&target)
        } else {
            None
        }
    });
    if let Some(sha) = commit {
        if !state.commits.contains(&sha) {
            state.commits.push(sha);
        }
    }
}

fn publish_activity(cwd: &std::path::Path) -> Result<()> {
    if session_git::common_dir(cwd).is_none() {
        return Ok(());
    }
    Command::new(std::env::current_exe()?)
        .args(["activity", "publish"])
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}
