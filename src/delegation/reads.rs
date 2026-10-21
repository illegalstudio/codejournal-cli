use super::{args, context, events, lifecycle, store, text};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

pub fn snapshot(args: &args::ReadArgs, other: bool) -> Result<Value> {
    ensure!(
        args.after.is_none_or(|after| after >= 0),
        "event cursor must not be negative"
    );
    let role = context::role(args.context.role.as_deref())?;
    store::transaction(|db| {
        let session = store::get(db, &args.context.id)?;
        context::authorize(&session, &role)?;
        let session = lifecycle::reconcile(db, session)?;
        let after = if args.full {
            0
        } else {
            args.after.unwrap_or_else(|| {
                session[format!("{role}_ack_seq")]
                    .as_i64()
                    .unwrap_or_default()
            })
        };
        let rows = events::list(db, &args.context.id, after, other.then_some(role.as_str()))?;
        let latest: i64 = db.query_row(
            "SELECT coalesce(max(seq),0) FROM events WHERE delegation_id=?",
            [&args.context.id],
            |r| r.get(0),
        )?;
        let next = rows
            .last()
            .and_then(|e| e["seq"].as_i64())
            .filter(|seq| *seq < latest);
        Ok(
            json!({"session":store::public(session),"events":rows,"latest_seq":latest,"next_after":next,
            "last_checkpoint":events::last(db,&args.context.id,"checkpoint")?}),
        )
    })
}

pub fn wait(args: args::WaitArgs, result_only: bool) -> Result<Value> {
    ensure!(
        args.timeout.is_finite() && (0.0..=300.0).contains(&args.timeout),
        "timeout must be between 0 and 300 seconds"
    );
    let deadline = Instant::now() + Duration::from_secs_f64(args.timeout);
    let mut read = args.read;
    loop {
        let mut value = snapshot(&read, !result_only)?;
        if super::terminal(&value["session"])
            || text(&value["session"], "status") == "stalled"
            || (!result_only && value["events"].as_array().is_some_and(|r| !r.is_empty()))
        {
            return Ok(value);
        }
        if Instant::now() >= deadline {
            value["timed_out"] = json!(true);
            return Ok(value);
        }
        if result_only {
            read.after = value["latest_seq"].as_i64();
            read.full = false;
        }
        std::thread::sleep(
            Duration::from_millis(100).min(deadline.saturating_duration_since(Instant::now())),
        );
    }
}

pub fn ack(args: args::ContextArgs, through: i64) -> Result<Value> {
    let role = context::role(args.role.as_deref())?;
    ensure!(through >= 0, "event sequence must not be negative");
    store::transaction(|db| {
        let session = store::get(db, &args.id)?;
        context::authorize(&session, &role)?;
        let latest: i64 = db.query_row(
            "SELECT coalesce(max(seq),0) FROM events WHERE delegation_id=?",
            [&args.id],
            |r| r.get(0),
        )?;
        ensure!(
            through <= latest,
            "cannot acknowledge events that do not exist"
        );
        let key = format!("{role}_ack_seq");
        let value = through.max(session[&key].as_i64().unwrap_or_default());
        let mut changes = json!({});
        changes[key] = json!(value);
        store::update(db, &args.id, changes)?;
        Ok(json!({"acknowledged_through":value}))
    })
}
