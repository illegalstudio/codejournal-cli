use super::{context, lifecycle, store, text};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
mod tail;

pub fn output(id: &str, lines: u16, max_chars: u32, raw: bool) -> Result<Value> {
    context::host_only()?;
    ensure!(
        (1..=200).contains(&lines),
        "diagnostic lines must be between 1 and 200"
    );
    let session = lifecycle::refresh(id)?;
    let directory = store::directory()?;
    let mut value = json!({"id":id, "status":session["status"], "attempt":session["attempt"],
        "source":"local", "provider_output_at":session["provider_output_at"],
        "last_guest_activity_at":session["last_activity"], "contact_at":session["guest_heartbeat"],
        "progress_state":if session["status"] == "waiting_for_host" {"waiting_for_host"} else if session["provider_output_at"].is_null() {"no_observed_provider_output"} else {"provider_output_observed"},
        "note":"Heartbeat is contact, not progress. Silence does not identify an API wait or a stuck provider. Summaries skip standalone stream markers; latest_event names the newest summarized event and its UTC time.",
        "capture_limit_bytes":2097152, "capture_policy":"rolling", "max_chars":max_chars, "streams":{}});
    let token = text(&session, "attempt_token");
    for stream in ["stdout", "stderr"] {
        let path = directory.join(format!("{id}-attempt-{}.{stream}.log", session["attempt"]));
        value["streams"][stream] = tail::read(&path, lines, max_chars, raw)?;
        if token.is_empty() {
            continue;
        }
        redact(&mut value["streams"][stream]["text"], token);
        if value["streams"][stream]["latest_event"].is_object() {
            redact(&mut value["streams"][stream]["latest_event"]["text"], token);
        }
    }
    crate::secret_redaction::value(&mut value);
    Ok(value)
}

fn redact(node: &mut Value, token: &str) {
    if let Some(content) = node.as_str() {
        *node = json!(content.replace(token, "[redacted]"));
    }
}
