use crate::{
    api::Api,
    watch_state,
    watches::{authorization, process},
};
use anyhow::Result;
use serde_json::{Value, json};
use std::{fmt, path::Path, time::Duration};

#[derive(Debug)]
pub struct StartupFailure(pub Value);

impl fmt::Display for StartupFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "watch {} startup was not confirmed{}; server state: {}; verified runner: {}; local authorization: {}. {} Runner log: {}",
            self.0["watch_id"].as_str().unwrap_or("?"),
            self.0["detail"].as_str().unwrap_or(""),
            self.0["server_status"].as_str().unwrap_or("unavailable"),
            self.0["runner_verified"],
            self.0["local_authorization"]
                .as_str()
                .unwrap_or("unavailable"),
            self.0["next"].as_str().unwrap_or(""),
            self.0["runner_log"].as_str().unwrap_or("")
        )
    }
}

impl std::error::Error for StartupFailure {}

pub fn failure(
    api: &Api,
    tenant: &str,
    path: &str,
    id: &str,
    log: &Path,
    detail: &str,
) -> Result<StartupFailure> {
    let unused = authorization::load(api, tenant, path, id).is_ok();
    let scope = crate::outbox::fingerprint(api.server(), tenant, &api.token);
    let pid = watch_state::read_pid(id, &scope)
        .ok()
        .filter(|pid| process::owns(*pid, id));
    let fresh = Api::with_timeout(api.server(), &api.token, Duration::from_secs(2))?;
    let status = fresh
        .get_fresh(&format!("{path}/{id}"))
        .ok()
        .and_then(|value| value["watch"]["status"].as_str().map(str::to_owned));
    Ok(StartupFailure(
        json!({"watch_id": id, "startup_confirmed": false,
        "started": if unused { json!(false) } else { Value::Null },
        "local_authorization": if unused { "unused" } else { "consumed_or_unavailable" },
        "runner_verified": pid.is_some(), "runner_pid": pid, "server_status": status,
        "runner_log": log, "detail": detail,
        "next": if unused { "Run cj sync on this host with the original account to recover this existing watch." }
            else { "Inspect cj watch output for this watch; consumed authorization cannot execute the command again." }}),
    ))
}
