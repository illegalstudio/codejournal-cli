use crate::{hook_events, hook_settings, outbox, session_state};
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};
use std::path::Path;

pub fn collect(agent: &str, path: &Path, settings: &Value) -> Result<Value> {
    let mut installed = hook_settings::installed(settings);
    installed.sort_unstable();
    let missing = hook_events::ALL
        .into_iter()
        .filter(|event| hook_events::spec(agent, event).is_some() && !installed.contains(event))
        .collect::<Vec<_>>();
    let since = Utc::now() - Duration::days(2);
    let mut sessions_seen = 0;
    let mut cursor_sessions_seen = 0;
    let mut last_event: Option<DateTime<Utc>> = None;
    for state in session_state::others()? {
        let timestamp = state
            .last_event_at
            .as_deref()
            .or_else(|| (!state.started_at.is_empty()).then_some(state.started_at.as_str()))
            .and_then(|text| DateTime::parse_from_rfc3339(text).ok())
            .map(|value| value.with_timezone(&Utc));
        if let Some(timestamp) = timestamp
            && timestamp >= since
            && (state.agent == agent || agent == "claude" && state.agent == "cursor")
        {
            if state.agent == "cursor" {
                cursor_sessions_seen += 1;
            } else {
                sessions_seen += 1;
            }
            last_event = Some(last_event.map_or(timestamp, |old| old.max(timestamp)));
        }
    }
    Ok(json!({
        "agent": agent,
        "settings": path,
        "script": std::env::current_exe()?.canonicalize()?,
        "present": path.parent().is_some_and(Path::exists),
        "installed": installed,
        "missing": missing,
        "queued_events": outbox::pending()?,
        "sessions_seen": sessions_seen,
        "cursor_sessions_seen": cursor_sessions_seen,
        "last_event_at": last_event.map(|time| time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
        "disabled_by_env": std::env::var("CODE_JOURNAL_HOOKS").as_deref() == Ok("off"),
    }))
}

pub fn invalid(agent: &str, path: &Path, error: &str) -> Value {
    let missing = hook_events::ALL
        .into_iter()
        .filter(|event| hook_events::spec(agent, event).is_some())
        .collect::<Vec<_>>();
    json!({"agent": agent, "settings": path, "error": error,
        "installed": [], "missing": missing})
}
