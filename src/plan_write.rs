use crate::api::Api;
use crate::{attribution, commands, input, refs};
use anyhow::Result;
use serde_json::json;

pub(crate) mod pending;
pub(crate) mod update;

pub(crate) fn noun(kind: &str) -> &str {
    if kind == "docs" { "doc" } else { "plan" }
}
pub(crate) fn path(tenant: &str, kind: &str, id: &str) -> String {
    format!("/api/v1/tenants/{tenant}/{kind}/{id}")
}
pub(crate) fn short(id: &str) -> String {
    id.replace('-', "").chars().take(8).collect()
}

#[allow(clippy::too_many_arguments)]
pub fn create(
    api: &Api,
    tenant: &str,
    project: Option<&str>,
    kind: &str,
    global: bool,
    title: String,
    body: Option<String>,
    body_file: Option<String>,
    status: Option<String>,
    not_before: Option<String>,
    raw_refs: Vec<String>,
    topics: Vec<String>,
    agent: Option<String>,
    json_mode: bool,
) -> Result<()> {
    anyhow::ensure!(
        topics.is_empty() || kind == "docs",
        "--topics is only supported for docs"
    );
    let body = input::body(body, body_file)?;
    crate::plan_validation::body(&body)?;
    let status =
        status.unwrap_or_else(|| if kind == "docs" { "current" } else { "active" }.to_owned());
    let base = if global {
        format!("/api/v1/tenants/{tenant}")
    } else {
        commands::path(tenant, project)?
    };
    let resource_id = uuid::Uuid::new_v4().to_string();
    let mut payload = json!({
        "id": resource_id, "title": title, "body": body, "status": status, "not_before": not_before,
        "refs": refs::parse_all(&raw_refs)?, "agent": attribution::agent(agent.as_deref()),
        "host": attribution::host(),
    });
    if !topics.is_empty() {
        payload["topics"] = json!(topics);
    }
    let result = match api.post(&format!("{base}/{kind}"), &payload) {
        Ok(result) => result,
        Err(error)
            if error
                .downcast_ref::<crate::request_outbox::QueuedWrite>()
                .is_some() =>
        {
            let request_id = error
                .downcast_ref::<crate::request_outbox::QueuedWrite>()
                .map(|queued| queued.0.as_str())
                .unwrap_or("");
            return crate::plan_show::size::emit(
                &json!({"queued": true, "synced": false, "id": request_id,
                "request_id": request_id, "resource_id": resource_id, "resource_type": kind,
                noun(kind): crate::api::sanitized(payload)}),
                &format!(
                    "Queued {kind} creation for synchronization (request {request_id}); resource ID: {resource_id}. Use this resource ID for --offline updates; it is not synchronized yet."
                ),
                json_mode,
            );
        }
        Err(error) => return Err(error),
    };
    pending::remember(api, tenant, &result)?;
    let item = &result[noun(kind)];
    crate::plan_show::size::emit(
        &json!({noun(kind): item, "queued": false, "warnings": []}),
        &format!(
            "Created {} {} ({}): {}",
            noun(kind),
            short(item["id"].as_str().unwrap_or("")),
            item["status"].as_str().unwrap_or(""),
            item["title"].as_str().unwrap_or("")
        ),
        json_mode,
    )
}
