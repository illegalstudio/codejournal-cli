use anyhow::Result;
use serde::Serialize;

#[derive(Serialize)]
pub struct Counts {
    pub current_scope: Option<usize>,
    pub other_scopes: Option<usize>,
    pub unbound: usize,
    pub unknown_scope: usize,
    pub delivery_enabled: bool,
    pub oldest_pending_at: Option<u64>,
}

pub fn inspect(origin: Option<&str>) -> Result<Counts> {
    let events = super::entries()?;
    let bound = events
        .iter()
        .filter(|(_, event)| super::origin::bound(event).is_some())
        .count();
    let current = origin.map(|origin| {
        events
            .iter()
            .filter(|(_, event)| super::origin::bound(event) == Some(origin))
            .count()
    });
    let oldest = events
        .iter()
        .filter_map(|(_, event)| {
            let timestamp = chrono::DateTime::parse_from_rfc3339(event["ts"].as_str()?)
                .ok()?
                .timestamp();
            u64::try_from(timestamp).ok()
        })
        .min();
    Ok(Counts {
        current_scope: current,
        other_scopes: current.map(|count| bound.saturating_sub(count)),
        unbound: events.len() - bound,
        unknown_scope: if origin.is_none() { bound } else { 0 },
        delivery_enabled: std::env::var("CODE_JOURNAL_HOOK_FLUSH").as_deref() != Ok("off"),
        oldest_pending_at: oldest,
    })
}

pub fn notices(counts: &Counts) -> Vec<String> {
    let mut notices = Vec::new();
    if counts.unbound > 0 {
        notices.push(format!("{} hook event(s) have no account binding and cannot sync; inspect cj outbox hooks list, then explicitly bind an event to its original account with cj outbox hooks bind ID --to-current-account, or discard it with cj outbox hooks drop ID", counts.unbound));
    }
    if counts.other_scopes.unwrap_or(0) > 0 {
        notices.push(format!("{} hook event(s) belong to other accounts or workspaces and remain preserved; use their original account/server", counts.other_scopes.unwrap_or(0)));
    }
    if counts.unknown_scope > 0 {
        notices.push(format!("{} bound hook event(s) cannot be matched while authentication is unavailable; inspect cj outbox hooks list", counts.unknown_scope));
    }
    if !counts.delivery_enabled && counts.current_scope.unwrap_or(0) > 0 {
        notices.push("Automatic hook delivery is disabled by CODE_JOURNAL_HOOK_FLUSH=off; explicit cj sync remains available.".to_owned());
    }
    notices
}
