use crate::session_state::SessionState;
use anyhow::Result;
use serde_json::{Value, json};
use std::path::Path;

pub fn completed(payload: &Value, state: &mut SessionState, base: &Value) -> Value {
    let mut base = base.clone();
    if let Some(id) = tool_id(payload).and_then(|id| state.approval_notifications.remove(id)) {
        base["resolved_notification_id"] = json!(id);
    }
    base
}

pub fn notify(
    payload: &Value,
    event: &str,
    cwd: &Path,
    state: &mut SessionState,
    reason: &str,
) -> Result<()> {
    let approval = (event == "PermissionRequest"
        || payload["notification_type"] == "permission_prompt")
        .then(|| tool_id(payload))
        .flatten();
    if approval.is_some_and(|tool| state.approval_notifications.contains_key(tool)) {
        return Ok(());
    }
    let id = approval.map(|tool| {
        let id = uuid::Uuid::new_v4().to_string();
        state
            .approval_notifications
            .insert(tool.to_owned(), id.clone());
        id
    });
    crate::hook_notify::notify(cwd, &state.agent, reason, id.as_deref())
}

fn tool_id(payload: &Value) -> Option<&str> {
    payload["tool_use_id"]
        .as_str()
        .or_else(|| payload["tool_call_id"].as_str())
}
