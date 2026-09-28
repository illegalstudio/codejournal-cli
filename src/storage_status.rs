use crate::api::Api;
use crate::{config::Config, outbox, outbox_flush, output, request_outbox};
use anyhow::Result;
use serde_json::json;

pub fn status(json_mode: bool) -> Result<()> {
    let path = crate::config::path()?;
    let config = Config::load().ok();
    let server = config.as_ref().map(|value| value.server.as_str());
    let tenant = config.as_ref().map(|value| value.tenant.as_str());
    let result = json!({
        "config_path": path, "database_path": null, "mode": if config.is_some() { "remote" } else { "unconfigured" },
        "remote_configured": config.is_some(), "server": server, "tenant": tenant,
        "pending_outbox": outbox::pending()? + request_outbox::pending()?, "last_sync_at": null, "notices": [],
    });
    let text = format!(
        "config:      {}{}\nstorage:     {}\nremote:      {}\noutbox:      {} pending",
        path.display(),
        if path.exists() { "" } else { " (missing)" },
        result["mode"].as_str().unwrap_or("unconfigured"),
        server.unwrap_or("not configured (run `cj login`)"),
        result["pending_outbox"]
    );
    output::emit(&result, &text, json_mode)
}

pub fn sync(api: &Api, json_mode: bool) -> Result<()> {
    api.get("/api/v1/me")?;
    let tenant = Config::load()?.tenant;
    let flushed = outbox_flush::run(api, &tenant)? + request_outbox::flush(api)?;
    output::emit(
        &json!({"synced": true, "flushed": flushed, "pending_outbox": outbox::pending()? + request_outbox::pending()?, "notices": []}),
        "Synchronized.",
        json_mode,
    )
}
