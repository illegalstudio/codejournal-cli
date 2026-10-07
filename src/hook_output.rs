use crate::{session_git, session_state};
use anyhow::Result;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

pub fn brief(cwd: &Path, source: &str, agent: &str, session: &str) -> Result<()> {
    let notice = crate::hook_auth::notice();
    let Some(cache) = crate::hook_brief::cache_path(cwd) else {
        if let Some(notice) = notice {
            context("SessionStart", &notice);
        }
        return Ok(());
    };
    let text = notice
        .is_none()
        .then(|| crate::hook_brief::online(source, agent, session))
        .flatten()
        .or_else(|| fs::read_to_string(&cache).ok());
    let notice = notice.or_else(crate::api_upgrade::notice);
    if let Some(text) = text {
        let message = notice
            .as_ref()
            .map_or_else(|| text.clone(), |notice| format!("{notice}\n\n{text}"));
        if crate::project_commands::archive::current().is_some() {
            crate::stdout::println!(
                "{}",
                json!({"journal_active": false, "hookSpecificOutput": {
                    "hookEventName": "SessionStart", "additionalContext": message
                }})
            );
        } else if source == "compact" {
            context(
                "SessionStart",
                &format!(
                    "The conversation was just compacted: before continuing, record with `cj add` any non-obvious fact you learned that the summary may have lost.\n\n{}",
                    message
                ),
            );
        } else {
            context("SessionStart", &message);
        }
        if let Some(parent) = cache.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(cache, text)?;
    } else if let Some(notice) = notice {
        context("SessionStart", &notice);
    }
    Ok(())
}

pub fn conflicts(warnings: &[String]) {
    if warnings.is_empty() {
        return;
    }
    context(
        "PreToolUse",
        &format!(
            "Code Journal: {}. Check the current content before editing and coordinate with the other session.",
            warnings.join("; ")
        ),
    );
}

pub fn stop(payload: &Value, cwd: &Path, state: &mut session_state::SessionState) {
    if state.journal_active != Some(true)
        || payload["stop_hook_active"].as_bool() == Some(true)
        || crate::project_commands::archive::current().is_some()
    {
        return;
    }
    let turn = if state.turn_started_at.is_empty() {
        &state.started_at
    } else {
        &state.turn_started_at
    };
    let moved = state
        .turn_head
        .as_ref()
        .is_some_and(|old| session_git::head(cwd).is_some_and(|head| &head != old));
    let logged = state.last_log_at.as_ref().is_some_and(|at| at >= turn);
    if (state.turn_files.is_empty() && !moved)
        || logged
        || state.reminded_turn.as_ref() == Some(turn)
    {
        return;
    }
    state.reminded_turn = Some(turn.to_owned());
    let what = if moved && !state.turn_files.is_empty() {
        format!(
            "changed {} file(s) and made commits",
            state.turn_files.len()
        )
    } else if moved {
        "made commits".to_owned()
    } else {
        format!("changed {} file(s)", state.turn_files.len())
    };
    crate::stdout::println!(
        "{}",
        json!({"decision": "block", "reason": format!("Code Journal: this turn {what} but no `cj log add` was recorded. Log it once now with `cj log add --title \"...\" --agent {}` and record any non-obvious fact with `cj add`. This reminder appears once per turn.", state.agent)})
    );
}

pub fn no_project() {
    crate::stdout::println!(
        "{}",
        json!({"journal_active": false, "hookSpecificOutput": {
            "hookEventName": "SessionStart", "additionalContext": crate::project_folder::notice()
        }})
    );
}

fn context(event: &str, text: &str) {
    crate::stdout::println!(
        "{}",
        json!({"hookSpecificOutput": {"hookEventName": event, "additionalContext": text}})
    );
}
