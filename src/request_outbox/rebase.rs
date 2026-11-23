//! Repairs one queued write whose base revision went stale. The caller supplies the base they
//! read and reviewed; nothing is sent, reordered or rebased on its own.
use super::{PendingRequest, entries, id_prefix, persist};
use crate::output;
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};

const FIELDS: [&str; 2] = ["based_on", "base"];

/// The body field that carries the base revision, when the write has exactly one.
pub fn base_field(request: &PendingRequest) -> Option<&'static str> {
    let body = request.body.as_ref()?.as_object()?;
    let mut present = FIELDS
        .into_iter()
        .filter(|field| body.get(*field).is_some_and(|value| !value.is_null()));
    let field = present.next()?;
    present.next().is_none().then_some(field)
}

fn shown(value: &Value) -> String {
    value
        .as_str()
        .map_or_else(|| value.to_string(), str::to_owned)
}

pub fn run(id: &str, base: &str, json_mode: bool) -> Result<()> {
    let prefix = id_prefix(id)?;
    let lock = super::delivery::delivery_lock()?;
    ensure!(
        fs2::FileExt::try_lock_exclusive(&lock).is_ok(),
        "outbox synchronization is running; retry later"
    );
    let mut matches = entries()?
        .into_iter()
        .filter(|(_, request)| request.id.replace('-', "").starts_with(&prefix));
    let (Some((path, mut request)), None) = (matches.next(), matches.next()) else {
        bail!("request ID is missing or ambiguous");
    };
    let Some(field) = base_field(&request) else {
        bail!(
            "queued write {} carries no base revision; inspect it with cj outbox list --body, then retry or drop it",
            request.id
        );
    };
    let body = request.body.as_mut().context("queued write has no body")?;
    let before = body[field].clone();
    let after = if before.is_number() {
        json!(
            base.parse::<u64>()
                .context("this write's base is a revision number")?
        )
    } else {
        json!(base)
    };
    ensure!(
        before != after,
        "queued write {} is already based on {}",
        request.id,
        shown(&before)
    );
    body[field] = after.clone();
    let replaces = body.get("body").is_some_and(|value| !value.is_null());
    request.retry_at = None;
    persist(&path, &request)?;
    let effect = if replaces {
        format!(
            " This write replaces the whole body, so it overwrites changes made after {} unless you already merged them into it.",
            shown(&before)
        )
    } else {
        String::new()
    };
    output::emit(
        &json!({"rebased": request.id, "method": request.method, "path": request.path,
            "field": field, "from": before, "to": after, "replaces_body": replaces, "sent": false}),
        &format!(
            "Rebased queued write {} ({} {}): {field} {} -> {}. It keeps its request ID and queue position and was not sent; run cj sync to deliver it.{effect}",
            request.id,
            request.method,
            request.path,
            shown(&before),
            shown(&after)
        ),
        json_mode,
    )
}
