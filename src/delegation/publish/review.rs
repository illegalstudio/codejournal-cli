use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OptionalExtension, params};

pub fn reply(db: &Connection, id: &str, role: &str, kind: &str, reply: Option<i64>) -> Result<()> {
    if kind != "answer" {
        return Ok(());
    }
    let reply = reply.context("answer requires --reply-to")?;
    let count:i64=db.query_row("SELECT count(*) FROM events WHERE delegation_id=? AND seq=? AND kind='question' AND sender!=?",
        params![id,reply,role],|r|r.get(0))?;
    ensure!(
        count == 1,
        "answer must reference the other endpoint's question in this delegation"
    );
    Ok(())
}

pub fn write(db: &Connection, id: &str) -> Result<()> {
    let seq:i64=db.query_row("SELECT seq FROM events WHERE delegation_id=? AND kind='checkpoint' AND sender='guest' ORDER BY seq DESC LIMIT 1",
        [id],|r|r.get(0)).optional()?.context("write result requires a guest diff checkpoint")?;
    let approvals:i64=db.query_row("SELECT count(*) FROM events AS a JOIN events AS q ON q.seq=a.reply_to WHERE a.delegation_id=? AND a.sender='host' AND a.kind='answer' AND a.seq>? AND q.delegation_id=a.delegation_id AND q.sender='guest' AND q.kind='question' AND q.seq>?",
        params![id,seq,seq],|r|r.get(0))?;
    ensure!(
        approvals > 0,
        "write result requires host review after the latest diff checkpoint"
    );
    Ok(())
}
