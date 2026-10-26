use super::{context, delivery, events, store, text};
mod diff;
mod review;
use anyhow::{Context, Result, ensure};
use rusqlite::params;
use serde_json::{Value, json};

pub fn run(args: super::args::PublishArgs) -> Result<Value> {
    let role = context::role(args.body.context.role.as_deref())?;
    let body = crate::input::optional_body(args.body.body, args.body.body_file)?;
    ensure!(
        body.chars().count() <= 100_000,
        "event body exceeds 100000 characters"
    );
    ensure!(
        matches!(args.kind.as_str(), "ready" | "status") || !body.is_empty(),
        "event requires a non-empty body"
    );
    ensure!(
        role == "guest" || !matches!(args.kind.as_str(), "ready" | "result" | "error"),
        "only a guest can publish this event"
    );
    let id = args.body.context.id;
    let changed = diff::inspect(&id, &role, &args.kind, args.body.idempotency_key.as_deref())?;
    store::transaction(|db| {
        let mut session = store::get(db, &id)?;
        context::authorize(&session, &role)?;
        if let Some(key) = &args.body.idempotency_key {
            let existing = db.query_row(
                "SELECT seq FROM events WHERE delegation_id=? AND sender=? AND idempotency_key=?",
                params![id, role, key],
                |row| row.get::<_, i64>(0),
            );
            if let Ok(seq) = existing {
                let event = events::list(db, &id, seq - 1, None)?.into_iter().next();
                return Ok(json!({"session": store::public(session), "event": event}));
            }
        }
        if super::terminal(&session) {
            let expected = if args.kind == "result" {
                "completed"
            } else {
                "failed"
            };
            if role == "guest"
                && text(&session, "status") == expected
                && text(&session, &args.kind) == body
            {
                return Ok(
                    json!({"session": store::public(session), "event": events::last(db, &id, &args.kind)?}),
                );
            }
            anyhow::bail!("delegation is already {}", session["status"]);
        }
        review::reply(db, &id, &role, &args.kind, args.body.reply_to)?;
        let pending = super::cursors::pending_host(db, &session)?;
        ensure!(
            role != "guest" || args.kind != "result" || pending["unread"] != true,
            "unread host messages through sequence {}; sync --role guest, address corrections and ack --through {} before publishing a result",
            pending["through"],
            pending["through"]
        );
        if role == "guest"
            && text(&session, "mode") == "write"
            && matches!(args.kind.as_str(), "checkpoint" | "result")
        {
            let changed = changed
                .as_deref()
                .context("workspace diff was not verified")?;
            if session["diff_detected_at"].is_null() {
                store::update(db, &id, json!({"diff_detected_at": super::now()}))?;
                store::event(
                    db,
                    &id,
                    "system",
                    "workspace_changed",
                    json!({"status": changed}),
                    None,
                    Some("first-workspace-change"),
                )?;
            }
            if args.kind == "result" {
                review::write(db, &id)?;
            }
        }
        let event = store::event(
            db,
            &id,
            &role,
            &args.kind,
            json!({"text":body}),
            args.body.reply_to,
            args.body.idempotency_key.as_deref(),
        )?;
        let status = match args.kind.as_str() {
            "ready" | "answer" => "running",
            "question" if role == "guest" => "waiting_for_host",
            "question" => "waiting_for_guest",
            "result" => "completed",
            "error" => "failed",
            _ => text(&session, "status"),
        };
        let now = super::now();
        let mut changes = json!({"status": status, "updated_at": now});
        if role == "guest" {
            changes["last_activity"] = json!(now);
        }
        changes[format!("{role}_heartbeat")] = json!(now);
        if matches!(args.kind.as_str(), "result" | "error") {
            changes[&args.kind] = json!(body);
        }
        store::update(db, &id, changes)?;
        session = store::get(db, &id)?;
        if matches!(
            args.kind.as_str(),
            "question" | "answer" | "checkpoint" | "result" | "error"
        ) {
            delivery::stage(db, &session, &args.kind, &role, &body)?;
        }
        Ok(
            json!({"event": event, "host_messages": super::cursors::pending_host(db,&session)?, "session": store::public(session)}),
        )
    })
}
