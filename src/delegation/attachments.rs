mod snapshot;
use super::{context, store, text};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
pub use snapshot::capture;

pub fn manifest(session: &Value) -> Result<Value> {
    let captured: Value = serde_json::from_str(text(session, "journal_context"))?;
    Ok(json!(
        captured["attachments"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|item| {
                let mut item = item.clone();
                item.as_object_mut().map(|map| map.remove("body"));
                item
            })
            .collect::<Vec<_>>()
    ))
}

pub fn read(id: &str, name: Option<&str>, offset: usize, limit: u16) -> Result<Value> {
    let session = store::get(&store::open()?, id)?;
    let role = context::role(None)?;
    context::authorize(&session, &role)?;
    let Some(name) = name else {
        return Ok(json!({"attachments":manifest(&session)?}));
    };
    let captured: Value = serde_json::from_str(text(&session, "journal_context"))?;
    let item = captured["attachments"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|item| item["name"] == name)
        .context("attached context not found")?;
    let body = item["body"].as_str().context("invalid context snapshot")?;
    let total = body.chars().count();
    ensure!(offset <= total, "context offset exceeds its length");
    let selected = body
        .chars()
        .skip(offset)
        .take(usize::from(limit))
        .collect::<String>();
    let end = offset + selected.chars().count();
    if role == "guest" {
        store::transaction(|db| {
            context::authorize(&store::get(db, id)?, &role)?;
            store::event(
                db,
                id,
                "guest",
                "context_read",
                json!({"name":name,"sha256":item["sha256"],"offset":offset,"end":end,"total_characters":total}),
                None,
                None,
            )?;
            Ok(())
        })?;
    }
    Ok(
        json!({"attachment":{"name":name,"sha256":item["sha256"],"source":item["source"],
        "offset":offset,"next_offset":if end < total {Some(end)} else {None},"total_characters":total,"body":selected}}),
    )
}

pub fn ensure_read(db: &rusqlite::Connection, session: &Value) -> Result<()> {
    let captured: Value = serde_json::from_str(text(session, "journal_context"))?;
    let mut statement = db.prepare("SELECT payload FROM events WHERE delegation_id=? AND sender='guest' AND kind='context_read'")?;
    let reads = statement
        .query_map([text(session, "id")], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .map(|text| serde_json::from_str::<Value>(&text))
        .collect::<serde_json::Result<Vec<_>>>()?;
    for attachment in captured["attachments"].as_array().into_iter().flatten() {
        let mut ranges = reads
            .iter()
            .filter(|read| {
                read["name"] == attachment["name"] && read["sha256"] == attachment["sha256"]
            })
            .filter_map(|read| Some((read["offset"].as_u64()?, read["end"].as_u64()?)))
            .collect::<Vec<_>>();
        ranges.sort_unstable();
        let mut covered = 0;
        for (start, end) in ranges {
            if start <= covered {
                covered = covered.max(end);
            }
        }
        ensure!(
            covered >= attachment["characters"].as_u64().unwrap_or(u64::MAX),
            "required context {} has unread ranges; read it with cj delegation context {} --name {} before publishing a result",
            attachment["name"],
            text(session, "id"),
            attachment["name"]
        );
    }
    Ok(())
}
