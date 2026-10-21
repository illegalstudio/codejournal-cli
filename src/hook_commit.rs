use crate::{session_git, session_state};
use serde_json::Value;
use std::path::Path;

pub fn record(payload: &Value, cwd: &Path, state: &mut session_state::SessionState) {
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
    let commit = session_git::commit_from_output(&target, output);
    if let Some(sha) = commit {
        if let Some(checkout) = session_git::checkout(&target) {
            state.commit_checkouts.insert(sha.clone(), checkout);
        }
        if !state.commits.contains(&sha) {
            state.commits.push(sha.clone());
            state.turn_commits.push(sha);
        }
    }
}
