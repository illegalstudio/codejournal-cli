use crate::{
    api::Api, brief_focus, brief_format, config::Config, outbox, project, session_git,
    session_state,
};
use anyhow::Result;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn brief(cwd: &Path, source: &str, agent: &str, session: &str) -> Result<()> {
    let Some(cache) = cache_path(cwd) else {
        return Ok(());
    };
    let text = online_brief(source, agent, session).or_else(|| fs::read_to_string(&cache).ok());
    if let Some(text) = text {
        if source == "compact" {
            context(
                "SessionStart",
                &format!(
                    "The conversation was just compacted: before continuing, record with `cj add` any non-obvious fact you learned that the summary may have lost.\n\n{}",
                    text
                ),
            );
        } else {
            context("SessionStart", &text);
        }
        if let Some(parent) = cache.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(cache, text)?;
    }
    Ok(())
}

fn online_brief(source: &str, agent: &str, session: &str) -> Option<String> {
    let config = Config::load().ok()?;
    let token = config.token().ok()?;
    let api = Api::with_timeout(&config.server, &token, Duration::from_secs(3)).ok()?;
    let preferred = std::env::var("CJ_PROJECT").ok();
    let slug = project::slug(preferred.as_deref()).ok()?;
    let query = reqwest::Url::parse_with_params(
        "http://local/",
        [
            ("limit", "10"),
            ("pinned_limit", "15"),
            ("log_limit", "5"),
            ("agent", agent),
            ("session", session),
        ],
    )
    .ok()?;
    let path = format!(
        "/api/v1/tenants/{}/projects/{slug}/brief?{}",
        config.tenant,
        query.query()?
    );
    let base = format!("/api/v1/tenants/{}/projects/{slug}", config.tenant);
    let result = brief_focus::load(
        &api,
        &base,
        &path,
        json!({
            "limit": 10, "pinned_limit": 15, "log_limit": 5, "agent": agent, "session": session,
        }),
    )
    .or_else(|error| {
        if !error.to_string().contains("404") {
            return Err(error);
        }
        api.post(
            &format!("/api/v1/tenants/{}/projects", config.tenant),
            &json!({"slug": slug, "name": slug}),
        )?;
        brief_focus::load(
            &api,
            &base,
            &path,
            json!({
                "limit": 10, "pinned_limit": 15, "log_limit": 5,
                "agent": agent, "session": session,
            }),
        )
    })
    .ok()?;
    let body = brief_format::render(&result, source == "compact", 10, 15, 5);
    Some(format!(
        "Code Journal brief for this repository (injected by the cj session hook; you do not need to run `cj brief` again unless it says so):\n\n{body}"
    ))
}

fn cache_path(cwd: &Path) -> Option<PathBuf> {
    let parent = outbox::directory().ok()?.parent()?.to_path_buf();
    let digest = Sha256::digest(cwd.to_string_lossy().as_bytes());
    Some(parent.join("briefs").join(format!("{digest:x}.txt")))
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
    if payload["stop_hook_active"].as_bool() == Some(true) {
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
    println!(
        "{}",
        json!({"decision": "block", "reason": format!("Code Journal: this turn {what} but no `cj log add` was recorded. Log it once now with `cj log add --title \"...\" --agent {}` and record any non-obvious fact with `cj add`. This reminder appears once per turn.", state.agent)})
    );
}

fn context(event: &str, text: &str) {
    println!(
        "{}",
        json!({"hookSpecificOutput": {"hookEventName": event, "additionalContext": text}})
    );
}
