use crate::api::Api;
use crate::{config::Config, outbox, outbox_flush, output, request_outbox};
use anyhow::Result;
use serde_json::json;

pub fn status(json_mode: bool, offline: bool) -> Result<()> {
    let startup_error = if offline {
        None
    } else {
        crate::request_sync::wake_configured()
            .err()
            .map(|error| error.to_string())
    };
    let path = crate::config::path()?;
    let config = Config::load().ok();
    let server = config.as_ref().map(|value| value.server.as_str());
    let tenant = config.as_ref().map(|value| value.tenant.as_str());
    let requests = request_outbox::entries()?;
    let mut by_server = std::collections::BTreeMap::<String, usize>::new();
    for (_, request) in requests {
        *by_server.entry(request.server).or_default() += 1;
    }
    let notice = crate::log_commits::pending(&[], false)
        .err()
        .map(|error| error.to_string());
    let mut automatic = crate::request_sync::status::inspect(config.as_ref())?;
    if let Some(error) = startup_error {
        automatic["last_error"] = json!(error);
        automatic["blocked"] = json!(true);
    }
    let hooks = outbox::pending()?;
    let writes = request_outbox::pending()?;
    let result = json!({
        "version": crate::api_version::VERSION, "build_revision": crate::api_version::REVISION, "build_dirty": crate::api_version::DIRTY,
        "config_path": path, "database_path": null, "mode": if config.is_some() { "remote" } else { "unconfigured" },
        "remote_configured": config.is_some(), "server": server, "tenant": tenant,
        "pending_outbox": hooks + writes, "pending_hook_events": hooks, "pending_writes": writes,
        "last_sync_at": automatic["last_sync_at"], "automatic_sync": automatic,
        "pending_writes_by_server": by_server, "notices": notice.iter().collect::<Vec<_>>(),
    });
    let text = format!(
        "cli:         {}\nconfig:      {}{}\nstorage:     {}\nremote:      {}\noutbox:      {} pending ({} writes, {} hook events)\n{}",
        crate::api_version::DISPLAY,
        path.display(),
        if path.exists() { "" } else { " (missing)" },
        result["mode"].as_str().unwrap_or("unconfigured"),
        server.unwrap_or("not configured (run `cj login`)"),
        result["pending_outbox"],
        writes,
        hooks,
        crate::request_sync::status::describe(&result["automatic_sync"])
    );
    let text = notice.map_or_else(
        || text.clone(),
        |notice| format!("{text}\nCommit tracking: {notice}"),
    );
    output::emit(&result, &text, json_mode)
}

pub fn sync(api: &Api, json_mode: bool) -> Result<()> {
    api.get("/api/v1/me")?;
    let tenant = Config::load()?.tenant;
    let hooks = outbox_flush::run(api, &tenant)?;
    let requests = request_outbox::flush(api);
    let resumed = crate::watches::recovery::resume(api, &tenant)?;
    let flushed = hooks + requests?;
    crate::request_sync::status::synchronized(api, &tenant)?;
    let reconciliation = crate::watches::recovery::reconcile(api, &tenant);
    let reconciled = reconciliation.is_ok();
    let mut notices = Vec::new();
    let lost = reconciliation.unwrap_or_else(|error| {
        notices.push(format!("Watch reconciliation deferred: {error}"));
        0
    });
    let other = request_outbox::entries()?
        .iter()
        .filter(|(_, request)| request.server != api.server())
        .count();
    if other > 0 {
        notices.push(format!(
            "{other} queued write(s) belong to other servers and remain pending; inspect them with cj outbox list"
        ));
    }
    let text = if notices.is_empty() {
        if resumed == 0 && lost == 0 {
            "Synchronized.".to_owned()
        } else {
            format!("Synchronized. Watches started: {resumed}; lost runners: {lost}.")
        }
    } else {
        format!("Synchronized.\n{}", notices.join("\n"))
    };
    output::emit(
        &json!({"synced": true, "flushed": flushed, "watches_started": resumed, "watches_lost": lost, "watches_reconciled": reconciled, "pending_outbox": outbox::pending()? + request_outbox::pending()?, "notices": notices}),
        &text,
        json_mode,
    )
}
