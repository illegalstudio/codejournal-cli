use super::{args::Action, store};
use anyhow::{Result, ensure};

pub fn id(value: &str) -> Result<String> {
    let prefix = value.replace('-', "").to_ascii_lowercase();
    ensure!(
        (8..=32).contains(&prefix.len()) && prefix.bytes().all(|b| b.is_ascii_hexdigit()),
        "invalid delegation ID; use a full UUID or a unique prefix of at least eight hexadecimal characters"
    );
    if super::context::role(None)? == "guest" {
        let own = std::env::var("CJ_DELEGATION_ID").unwrap_or_default();
        ensure!(
            own.replace('-', "").starts_with(&prefix),
            "guest attempt is no longer authorized"
        );
        return Ok(own);
    }
    store::read(|db| {
        let mut query =
            db.prepare("SELECT id FROM delegations WHERE replace(id, '-', '') LIKE ? LIMIT 2")?;
        let rows = query
            .query_map([format!("{prefix}%")], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        ensure!(
            rows.len() == 1,
            "delegation ID {value} is missing or ambiguous; use cj delegation list and select a full UUID"
        );
        Ok(rows[0].clone())
    })
}

pub fn action(action: &mut Action, full: bool) -> Result<()> {
    let value = match action {
        Action::Sync(r) | Action::Status(r) => {
            r.full = full;
            Some(&mut r.context.id)
        }
        Action::Wait(w) | Action::Result(w) => {
            w.read.full = full;
            Some(&mut w.read.context.id)
        }
        Action::Ack { context, .. } => Some(&mut context.id),
        Action::Publish(p) => Some(&mut p.body.context.id),
        Action::Send(b) | Action::Answer(b) | Action::Checkpoint(b) => Some(&mut b.context.id),
        Action::Recover { id, .. }
        | Action::Integrate { id }
        | Action::Cancel { id, .. }
        | Action::Purge { id, .. }
        | Action::McpConfig { id }
        | Action::Output { id, .. } => Some(id),
        _ => None,
    };
    if let Some(value) = value {
        *value = id(value)?;
    }
    Ok(())
}
