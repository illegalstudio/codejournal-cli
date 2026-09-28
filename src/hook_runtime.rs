use crate::{
    hook_commit, hook_files, hook_output, hook_session::queue, session_state::SessionState,
};
use anyhow::Result;
use serde_json::{Value, json};
use std::path::Path;

pub fn tool(
    payload: &Value,
    cwd: &Path,
    state: &mut SessionState,
    base: &Value,
    now: &str,
) -> Result<()> {
    hook_commit::record(payload, cwd, state);
    if let Some(id) = delegation_link(payload) {
        queue(base, "delegation_link", "delegation_id", json!(id))?;
    }
    let files = hook_files::edited(payload, cwd, state.root.as_deref());
    if files.is_empty() {
        if state.waiting || stale(state.last_event_at.as_deref(), 60) {
            queue(base, "tool", "", Value::Null)?;
            state.last_event_at = Some(now.to_owned());
        }
    } else {
        for file in files {
            if !state.turn_files.contains(&file) {
                state.turn_files.push(file.clone());
            }
            state.files.insert(file.clone(), now.to_owned());
            queue(base, "edit", "file", json!(file))?;
        }
        state.last_event_at = Some(now.to_owned());
    }
    state.waiting = false;
    Ok(())
}

pub fn waiting(
    event: &str,
    payload: &Value,
    cwd: &Path,
    state: &mut SessionState,
    base: &Value,
    now: &str,
) -> Result<()> {
    let notification_type = payload["notification_type"].as_str();
    if event == "Notification"
        && notification_type.is_some_and(|value| {
            ![
                "permission_prompt",
                "idle_prompt",
                "elicitation_dialog",
                "elicitation_url_dialog",
                "agent_needs_input",
            ]
            .contains(&value)
        })
    {
        return Ok(());
    }
    let reason = if event == "PermissionRequest" {
        format!(
            "approval needed for {}",
            payload["tool_name"].as_str().unwrap_or("a tool")
        )
    } else {
        payload["message"]
            .as_str()
            .or_else(|| payload["title"].as_str())
            .unwrap_or("needs your attention")
            .to_owned()
    };
    state.waiting = true;
    queue(
        base,
        "waiting",
        "reason",
        json!(reason.chars().take(200).collect::<String>()),
    )?;
    if stale(state.notified_at.as_deref(), 300) {
        state.notified_at = Some(now.to_owned());
        crate::hook_notify::notify(cwd, &state.agent, &reason)?;
    }
    Ok(())
}

pub fn stop(payload: &Value, cwd: &Path, state: &mut SessionState, base: &Value) -> Result<()> {
    queue(base, "turn", "", Value::Null)?;
    if state.delegation_id.is_none() {
        hook_output::stop(payload, cwd, state);
    }
    Ok(())
}

fn stale(when: Option<&str>, seconds: i64) -> bool {
    when.and_then(|text| chrono::DateTime::parse_from_rfc3339(text).ok())
        .is_none_or(|then| chrono::Utc::now().signed_duration_since(then).num_seconds() >= seconds)
}

fn delegation_link(payload: &Value) -> Option<String> {
    let command = payload["tool_input"]["command"]
        .as_str()
        .or_else(|| payload["tool_input"]["cmd"].as_str())?;
    if !command.contains("delegate") || !command.contains(" start") {
        return None;
    }
    let output = payload["tool_response"]["stdout"]
        .as_str()
        .or_else(|| payload["tool_output"].as_str())?;
    let pattern = regex::Regex::new(r#""id\\?"\s*:\s*\\?"([0-9a-f-]{36})"#).ok()?;
    pattern
        .captures(output)?
        .get(1)
        .map(|value| value.as_str().to_owned())
}
