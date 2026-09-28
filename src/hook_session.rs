use crate::{
    attribution, git, hook_files, hook_output, hook_runtime, outbox, project, session_git,
    session_state,
};
use anyhow::Result;
use serde_json::{Value, json};
use std::path::Path;
use std::process::{Command, Stdio};

pub fn handle(event: &str, payload: &Value, session: &str, cwd: &Path) -> Result<()> {
    let mut state = session_state::load(session)?;
    let now = session_state::now();
    if state.started_at.is_empty() {
        state.started_at = now.clone();
        state.agent = runtime_agent(payload);
        state.session_id = session.to_owned();
        state.root = git::root().map(|path| path.display().to_string());
        state.repo_common = session_git::common_dir(cwd);
        state.cwd = cwd.display().to_string();
        if std::env::var("ME_DELEGATION_ROLE").as_deref() == Ok("guest") {
            if let Ok(id) = std::env::var("ME_DELEGATION_ID") {
                state.delegation_id = Some(id);
            }
        }
    }
    let preferred = std::env::var("CJ_PROJECT").ok();
    let project = state
        .root
        .as_ref()
        .and_then(|_| project::slug(preferred.as_deref()).ok());
    let base = json!({"session": session, "agent": state.agent, "project": project,
        "project_explicit": preferred.is_some(), "checkout_path": state.root,
        "host": attribution::host(), "cwd": state.cwd, "ts": now});
    if state.delegation_id.is_some() && state.last_event_at.is_none() {
        queue(&base, "guest", "delegation_id", json!(state.delegation_id))?;
        state.last_event_at = Some(now.clone());
    }
    match event {
        "SessionStart" => {
            state.cwd = cwd.display().to_string();
            state.ended_at = None;
            state.turn_started_at = now.clone();
            state.turn_head = session_git::head(cwd);
            state.turn_files.clear();
            state.reminded_turn = None;
            let source = payload["source"].as_str().unwrap_or("startup");
            queue(&base, "start", "source", json!(source))?;
            session_state::save(session, &state)?;
            hook_output::brief(cwd, source, &state.agent, session)?;
        }
        "UserPromptSubmit" => {
            state.turn_started_at = now.clone();
            state.turn_head = session_git::head(cwd);
            state.turn_files.clear();
            state.reminded_turn = None;
            state.waiting = false;
            queue(&base, "prompt", "", Value::Null)?;
        }
        "PreToolUse" => {
            if let Some(root) = &state.root {
                let files = hook_files::edited(payload, cwd, Some(root));
                hook_output::conflicts(&hook_files::conflicts(session, root, &files));
            }
        }
        "PostToolUse" => hook_runtime::tool(payload, cwd, &mut state, &base, &now)?,
        "Notification" | "PermissionRequest" => {
            hook_runtime::waiting(event, payload, cwd, &mut state, &base, &now)?
        }
        "Stop" => hook_runtime::stop(payload, cwd, &mut state, &base)?,
        "PreCompact" => {
            queue(&base, "compact", "", Value::Null)?;
        }
        "SessionEnd" => {
            state.ended_at = Some(now.clone());
            queue(&base, "end", "reason", payload["reason"].clone())?;
        }
        _ => {}
    }
    state.last_event_at = Some(now);
    session_state::save(session, &state)?;
    if event != "PreToolUse" {
        let _ = outbox::spawn_flush();
    }
    if matches!(event, "SessionStart" | "PostToolUse") && state.root.is_some() {
        let _ = Command::new(std::env::current_exe()?)
            .args(["activity", "publish"])
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
    Ok(())
}

fn runtime_agent(payload: &Value) -> String {
    if payload["cursor_version"].as_str().is_some() || std::env::var("CURSOR_VERSION").is_ok() {
        "cursor".to_owned()
    } else {
        attribution::agent(None)
    }
}

#[cfg(test)]
mod tests {
    use super::runtime_agent;
    use serde_json::json;

    #[test]
    fn cursor_payload_overrides_inherited_agent_identity() {
        assert_eq!(
            runtime_agent(&json!({"cursor_version": "2026.09"})),
            "cursor"
        );
    }
}

pub(crate) fn queue(base: &Value, kind: &str, field: &str, value: Value) -> Result<()> {
    let mut item = base.clone();
    item["type"] = json!(kind);
    if !field.is_empty() {
        item[field] = value;
    }
    outbox::enqueue(item)?;
    Ok(())
}
