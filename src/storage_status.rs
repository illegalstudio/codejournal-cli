use crate::api::Api;
use crate::{config::Config, outbox, outbox_flush, output, request_outbox};
use anyhow::Result;
use serde_json::json;

pub fn status(json_mode: bool, offline: bool, server_override: Option<&str>) -> Result<()> {
    let path = crate::config::path()?;
    let config = Config::load().ok();
    let server = server_override.or_else(|| config.as_ref().map(|value| value.server.as_str()));
    let tenant = config.as_ref().map(|value| value.tenant.as_str());
    let startup_error = if offline
        || !crate::request_sync::enabled()
        || (outbox::pending()? == 0 && request_outbox::pending()? == 0)
    {
        None
    } else {
        config
            .as_ref()
            .map(|config| {
                let server = server.unwrap_or(&config.server);
                let api = Api::new(server, &config.token_for(server)?)?;
                crate::request_sync::wake(&api, &config.tenant)
            })
            .transpose()
            .err()
            .map(|error| error.to_string())
    };
    let requests = request_outbox::entries()?;
    let mut by_server = std::collections::BTreeMap::<String, usize>::new();
    for (_, request) in requests {
        *by_server.entry(request.server).or_default() += 1;
    }
    let notice = crate::log_commits::notice(&[], false)?;
    let mut automatic = crate::request_sync::status::inspect(config.as_ref(), server)?;
    if let Some(error) = startup_error {
        automatic["last_error"] = json!(error);
        automatic["blocked"] = json!(true);
    }
    let hooks = outbox::pending()?;
    let writes = request_outbox::pending()?;
    if !offline {
        crate::update_check::refresh_if_due();
    }
    let update = crate::update_check::available();
    let result = json!({
        "update_available": update,
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
    let text = update
        .as_ref()
        .and_then(crate::update_check::line)
        .map_or_else(|| text.clone(), |line| format!("{text}\n{line}"));
    output::emit(&result, &text, json_mode)
}

pub fn sync(api: &Api, json_mode: bool) -> Result<()> {
    api.get("/api/v1/me")?;
    let tenant = Config::load()?.tenant;
    let hooks = outbox_flush::run(api, &tenant)?;
    let requests = request_outbox::flush(api);
    let resumed = crate::watches::recovery::resume(api, &tenant)?;
    let flushed = hooks.sent
        + requests.map_err(|error| crate::request_sync::blockage::explain(api, error))?;
    crate::request_sync::status::synchronized(api, &tenant, hooks.expired)?;
    let reconciliation = crate::watches::recovery::reconcile(api, &tenant);
    let reconciled = reconciliation.is_ok();
    let mut notices = Vec::new();
    if hooks.expired > 0 {
        notices.push(format!(
            "{} hook event(s) expired before delivery.",
            hooks.expired
        ));
    }
    if hooks.unbound_removed > 0 {
        notices.push(outbox::expiry::notice(hooks.unbound_removed));
    }
    let origin = outbox::fingerprint(api.server(), &tenant, &api.token);
    let hook_counts = outbox::diagnostics::inspect(Some(&origin))?;
    notices.extend(outbox::diagnostics::notices(&hook_counts));
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
        &json!({"synced": true, "flushed": flushed, "expired_hooks": hooks.expired, "unbound_hooks_removed": hooks.unbound_removed, "watches_started": resumed, "watches_lost": lost, "watches_reconciled": reconciled, "pending_outbox": outbox::pending()? + request_outbox::pending()?, "hook_events": hook_counts, "notices": notices}),
        &text,
        json_mode,
    )
}
