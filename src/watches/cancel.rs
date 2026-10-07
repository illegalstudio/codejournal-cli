use super::{authorization, process};
use crate::{api::Api, attribution, outbox, output, watch_format, watch_state};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

pub fn run(api: &Api, tenant: &str, path: &str, prefix: &str, json_output: bool) -> Result<()> {
    let pending = authorization::pending(api, tenant)?
        .into_iter()
        .filter(|(id, _)| {
            id.replace('-', "")
                .starts_with(&prefix.replace('-', "").to_lowercase())
        })
        .collect::<Vec<_>>();
    if api.offline() {
        return cancel_pending(api, &pending, json_output);
    }
    let query = reqwest::Url::parse_with_params("http://local/", [("host", attribution::host())])?;
    let resolved = match api.get_fresh(&format!("{path}/{prefix}?{}", query.query().unwrap_or("")))
    {
        Ok(value) => value,
        Err(_) if pending.len() == 1 => return cancel_pending(api, &pending, json_output),
        Err(error) => return Err(error),
    };
    let watch = &resolved["watch"];
    let id = watch["id"].as_str().context("watch ID missing")?;
    if !matches!(
        watch["status"].as_str(),
        Some("starting" | "running" | "lost")
    ) {
        return output::emit(
            &json!({"watch": watch, "cancelled": false}),
            &watch_format::cancel(watch, false),
            json_output,
        );
    }
    let project = watch["project_slug"]
        .as_str()
        .context("watch project missing")?;
    let endpoint = format!("{}/watches", super::endpoint(tenant, project));
    let result = api.patch(
        &format!("{endpoint}/{id}"),
        &json!({"status": "cancelled", "host": attribution::host()}),
    );
    if let Err(error) = &result
        && error
            .downcast_ref::<crate::request_outbox::QueuedWrite>()
            .is_none()
    {
        return result.map(|_| ());
    }
    let scope = outbox::fingerprint(api.server(), tenant, &api.token);
    let stopped = watch_state::read_pid(id, &scope).is_ok_and(|pid| process::terminate(pid, id));
    if authorization::load(api, tenant, &endpoint, id).is_ok() {
        authorization::remove(id);
    }
    let result = match result {
        Ok(value) => value,
        Err(error) => {
            let queued = error
                .downcast_ref::<crate::request_outbox::QueuedWrite>()
                .context("cancellation failed")?;
            return deferred(&queued.0, id, stopped, json_output);
        }
    };
    let updated = &result["watch"];
    let cancelled = updated["status"] == "cancelled";
    output::emit(
        &json!({"watch": updated, "cancelled": cancelled, "local_process_stopped": stopped}),
        &watch_format::cancel(updated, cancelled),
        json_output,
    )
}

fn cancel_pending(
    api: &Api,
    pending: &[(String, authorization::Authorization)],
    json_output: bool,
) -> Result<()> {
    let [(id, authorized)] = pending else {
        bail!(
            "cancel needs one locally authorized pending watch; retry online or use its complete ID"
        );
    };
    let mut deferred_api = api.clone();
    deferred_api.set_offline(true);
    let result = deferred_api.patch(
        &format!("{}/{id}", authorized.endpoint),
        &json!({"status": "cancelled", "host": attribution::host()}),
    );
    match result {
        Err(error) => {
            let queued = error
                .downcast_ref::<crate::request_outbox::QueuedWrite>()
                .context("cancellation was not queued")?;
            authorization::remove(id);
            deferred(&queued.0, id, false, json_output)
        }
        Ok(value) => output::json(&value),
    }
}

fn deferred(request: &str, id: &str, stopped: bool, json_output: bool) -> Result<()> {
    let receipt: Value = json!({"queued": true, "cancelled": false, "watch_id": id,
        "request_id": request, "local_process_stopped": stopped, "pending_execution_removed": true});
    output::emit(
        &receipt,
        "Watch cancellation queued. No unused local authorization remains. Run cj sync on this host.",
        json_output,
    )?;
    bail!("remote watch cancellation is pending synchronization")
}
