use crate::{activity_state::DeliveryState, api::Api, output, request_outbox, secret_redaction};
use anyhow::Result;
use serde_json::{Value, json};

pub fn publish(api: &Api, endpoint: &str, body: &Value) -> Result<Value> {
    let mut body = body.clone();
    output::record_masking(secret_redaction::value(&mut body));
    let state = DeliveryState::lock(api.server(), &api.token, endpoint, &body)?;
    let tenant_root = endpoint.split("/projects/").next().unwrap_or(endpoint);
    let pending = request_outbox::entries()?
        .into_iter()
        .filter(|(_, request)| {
            request.server == api.server()
                && request
                    .path
                    .starts_with(&format!("{tenant_root}/projects/"))
                && request.path.ends_with("/checkout-activity")
                && request.body.as_ref().is_some_and(|queued| {
                    queued["host"] == body["host"] && queued["path"] == body["path"]
                })
        })
        .last();
    if let Some((_, latest)) = pending {
        if latest.path == endpoint && latest.body.as_ref() == Some(&body) {
            return Ok(json!({"changed": false, "skipped": true, "pending": true}));
        }
        // Keep state transitions ordered behind the previously queued snapshot.
        let mut queued = api.clone();
        queued.set_offline(true);
        return queued.post(endpoint, &body);
    }
    if state.recently_delivered() {
        return Ok(json!({"changed": false, "skipped": true, "pending": false}));
    }
    let result = api.post(endpoint, &body)?;
    if !api.offline() {
        state.acknowledge()?;
    }
    Ok(result)
}
