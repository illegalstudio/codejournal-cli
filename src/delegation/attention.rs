use super::{context, store, text};
use anyhow::Result;
use serde_json::{Value, json};
use std::{
    io::{IsTerminal, Read},
    path::Path,
};

pub fn settings(binary: &Path, id: &str) -> Value {
    let command = format!(
        "if [ \"${{CJ_DELEGATION_ID:-}}\" = {} ]; then {} delegation attention {}; else echo '{{}}'; fi",
        shell_words::quote(id),
        shell_words::quote(&binary.to_string_lossy()),
        shell_words::quote(id)
    );
    json!({"hooks":{"PreToolUse":[{"matcher":".*","hooks":[{"type":"command","command":command,"timeout":5}]}]}})
}

pub fn response(id: &str) -> Result<Value> {
    let stdin = std::io::stdin();
    if !stdin.is_terminal() {
        std::io::copy(&mut stdin.take(1_048_576), &mut std::io::sink())?;
    }
    if std::env::var("CJ_DELEGATION_ID").as_deref() != Ok(id) || context::role(None)? != "guest" {
        return Ok(json!({}));
    }
    Ok(match notice(id)? {
        Some(message) => {
            json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","additionalContext":message}})
        }
        None => json!({}),
    })
}

pub fn current_notice() -> Result<Option<String>> {
    if context::role(None)? != "guest" {
        return Ok(None);
    }
    let Ok(id) = std::env::var("CJ_DELEGATION_ID") else {
        return Ok(None);
    };
    notice(&id)
}

fn notice(id: &str) -> Result<Option<String>> {
    let role = context::role(None)?;
    let found = store::read(|db| {
        let session = store::get(db, id)?;
        context::authorize(&session, &role)?;
        let pending = super::cursors::pending_host(db, &session)?;
        if pending["unread"] != true {
            return Ok(None);
        }
        let after = session["guest_ack_seq"].as_i64().unwrap_or_default();
        let mut statement = db.prepare("SELECT seq,payload FROM events WHERE delegation_id=? AND sender='host' AND kind IN ('message','question','answer') AND seq>? ORDER BY seq LIMIT 5")?;
        let rows = statement
            .query_map(rusqlite::params![id, after], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut message = format!(
            "Pending host corrections ({} unread, oldest {} seconds):\n",
            pending["unread_count"], pending["oldest_unread_age_seconds"]
        );
        for (seq, payload) in rows {
            let payload: Value = serde_json::from_str(&payload)?;
            message.push_str(&format!(
                "{seq}: {}\n",
                text(&payload, "text")
                    .chars()
                    .take(1200)
                    .collect::<String>()
            ));
        }
        message.push_str(&format!("Read all corrections with cj delegation sync {id} --role guest --full --events-only before dependent work; acknowledge only after inspecting them."));
        let through = pending["through"].as_i64().unwrap_or_default();
        let key = format!("host-attention-{through}");
        let recorded: i64 = db.query_row("SELECT count(*) FROM events WHERE delegation_id=? AND sender='system' AND idempotency_key=?",rusqlite::params![id,key],|row|row.get(0))?;
        Ok(Some((message, through, recorded == 0)))
    })?;
    let Some((message, through, record)) = found else {
        return Ok(None);
    };
    if role == "guest" && record {
        store::transaction(|db| {
            let session = store::get(db, id)?;
            context::authorize(&session, &role)?;
            if session["guest_ack_seq"].as_i64().unwrap_or_default() < through {
                store::event(
                    db,
                    id,
                    "system",
                    "host_attention",
                    json!({"through":through,"channel":"tool_boundary"}),
                    None,
                    Some(&format!("host-attention-{through}")),
                )?;
            }
            Ok(())
        })?;
    }
    Ok(Some(message))
}
