use crate::{api::Api, attribution};
use anyhow::Result;
use serde_json::{Value, json};
use std::time::Duration;

pub fn client(api: &Api) -> Result<Api> {
    let mut fresh = Api::with_timeout(api.server(), &api.token, Duration::from_secs(3))?;
    fresh.set_offline(api.offline());
    Ok(fresh)
}

pub fn renew(api: &Api, path: &str, id: &str, runner: &str) -> Result<Value> {
    anyhow::ensure!(
        !api.offline(),
        "watch heartbeat requires an online response"
    );
    let result = api.post_noqueue(
        &format!("{path}/{id}/heartbeat"),
        &json!({"runner_id": runner, "host": attribution::host()}),
    )?;
    anyhow::ensure!(
        result["active"].is_boolean() && result["watch"]["id"] == id,
        "invalid fresh watch heartbeat response"
    );
    Ok(result)
}
