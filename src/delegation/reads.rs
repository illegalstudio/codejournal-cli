use super::{args, context, events, lifecycle, store};
use anyhow::{Result, ensure};
use serde_json::{Value, json};

pub fn snapshot(args: &args::ReadArgs, other: bool) -> Result<Value> {
    ensure!(
        args.after.is_none_or(|after| after >= 0),
        "event cursor must not be negative"
    );
    let role = context::role(args.context.role.as_deref())?;
    context::authorize(&store::get(&store::open()?, &args.context.id)?, &role)?;
    lifecycle::refresh(&args.context.id)?;
    store::read(|db| {
        let session = store::get(db, &args.context.id)?;
        context::authorize(&session, &role)?;
        let after = if args.full {
            args.after.unwrap_or(0)
        } else {
            args.after.unwrap_or_else(|| {
                session[format!("{role}_ack_seq")]
                    .as_i64()
                    .unwrap_or_default()
            })
        };
        let mut rows = events::list(db, &args.context.id, after, other.then_some(role.as_str()))?;
        if !args.full {
            rows.truncate(25);
        }
        let latest: i64 = db.query_row(
            "SELECT coalesce(max(seq),0) FROM events WHERE delegation_id=?",
            [&args.context.id],
            |r| r.get(0),
        )?;
        let next = rows
            .last()
            .and_then(|e| e["seq"].as_i64())
            .filter(|seq| *seq < latest);
        let host_messages = super::cursors::pending_host(db, &session)?;
        Ok(
            json!({"session":store::public(session),"events":rows,"latest_seq":latest,"next_after":next,
            "last_checkpoint":events::last(db,&args.context.id,"checkpoint")?,
            "result_event":events::last(db,&args.context.id,"result")?,
            "host_messages":host_messages}),
        )
    })
}

pub fn wait(args: args::WaitArgs, result_only: bool) -> Result<Value> {
    super::waiting::run(args, result_only)
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
