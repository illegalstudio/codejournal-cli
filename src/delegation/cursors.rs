use super::{context, store};
use anyhow::Result;
use rusqlite::{OptionalExtension, params};
use serde_json::json;

pub fn after(id: &str, role: &str) -> Result<i64> {
    store::read(|db| {
        let session = store::get(db, id)?;
        context::authorize(&session, role)?;
        let seen = db
            .query_row(
                "SELECT seq FROM wait_cursors WHERE delegation_id=? AND role=?",
                params![id, role],
                |r| r.get::<_, i64>(0),
            )
            .optional()?
            .unwrap_or(0);
        Ok(seen.max(session[format!("{role}_ack_seq")].as_i64().unwrap_or(0)))
    })
}

pub fn delivered(id: &str, role: &str, seq: i64) -> Result<()> {
    store::transaction(|db| {
        context::authorize(&store::get(db, id)?, role)?;
        db.execute(
            "INSERT INTO wait_cursors(delegation_id,role,seq) VALUES(?,?,?)
            ON CONFLICT(delegation_id,role) DO UPDATE SET seq=max(seq,excluded.seq)",
            params![id, role, seq],
        )?;
        Ok(())
    })
}

pub fn pending_host(
    db: &rusqlite::Connection,
    session: &serde_json::Value,
) -> Result<serde_json::Value> {
    let latest: i64 = db.query_row("SELECT coalesce(max(seq),0) FROM events WHERE delegation_id=? AND sender='host' AND kind IN ('message','answer','question')",
        [super::text(session,"id")], |r| r.get(0))?;
    let unread = latest > session["guest_ack_seq"].as_i64().unwrap_or(0);
    let (count, oldest): (i64, Option<f64>) = db.query_row("SELECT count(*),min(created_at) FROM events WHERE delegation_id=? AND sender='host' AND kind IN ('message','answer','question') AND seq>?",
        params![super::text(session,"id"),session["guest_ack_seq"].as_i64().unwrap_or(0)], |row| Ok((row.get(0)?,row.get(1)?)))?;
    let attention: Option<f64> = db.query_row(
        "SELECT max(created_at) FROM events WHERE delegation_id=? AND kind='host_attention'",
        [super::text(session, "id")],
        |row| row.get(0),
    )?;
    Ok(
        json!({"unread":unread,"through":latest,"unread_count":count,"oldest_unread_at":oldest,
            "oldest_unread_age_seconds":oldest.map(|at|(super::now()-at).max(0.0)),"last_tool_boundary_notice_at":attention,
            "delivery_state":if unread {"pending_guest_acknowledgement"} else {"acknowledged"},
            "hint":if unread {"Read host corrections with sync --role guest, address them, then ack --through SEQ before publishing a result."} else {""}}),
    )
}
