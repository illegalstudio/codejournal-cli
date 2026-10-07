use crate::api::Api;
use crate::watch_args::WatchAction;
use crate::{attribution, output, project_bootstrap, watch_format, watch_runner};
use anyhow::Result;

pub(crate) mod authorization;
mod cancel;
pub(crate) mod execution;
pub(crate) mod launch;
pub(crate) mod lease;
pub(crate) mod process;
pub(crate) mod recovery;
mod start;

pub fn endpoint(tenant: &str, project_name: &str) -> String {
    format!("/api/v1/tenants/{tenant}/projects/{project_name}")
}

pub fn run(
    api: &Api,
    server: &str,
    tenant: &str,
    project_name: Option<&str>,
    action: WatchAction,
    json_output: bool,
) -> Result<()> {
    let global = matches!(
        &action,
        WatchAction::List { .. } | WatchAction::Cancel { .. }
    );
    let project = if global {
        None
    } else if project_name.is_none() && matches!(&action, WatchAction::Start { .. }) {
        Some(project_bootstrap::ensure(api, tenant, false)?)
    } else {
        Some(project_bootstrap::resolved_slug(api, tenant, project_name)?)
    };
    let path = project.as_ref().map_or_else(
        || format!("/api/v1/tenants/{tenant}/watches"),
        |slug| format!("{}/watches", endpoint(tenant, slug)),
    );
    match action {
        WatchAction::Start {
            title,
            notify_on,
            timeout,
            agent,
            command,
        } => start::run(
            api,
            server,
            tenant,
            project.as_deref().unwrap_or(""),
            &path,
            title,
            notify_on,
            timeout,
            agent,
            command,
            json_output,
        ),
        WatchAction::List { all } => {
            let result = api.get(&local_list_path(&path, all)?)?;
            output::emit(&result, &watch_format::list(&result), json_output)
        }
        WatchAction::Cancel { id } => cancel::run(api, tenant, &path, &id, json_output),
        WatchAction::Run { id } => watch_runner::run(api, tenant, &path, &id),
    }
}

fn local_list_path(path: &str, all: bool) -> Result<String> {
    let host = attribution::host();
    let query = reqwest::Url::parse_with_params(
        "http://local/",
        [
            ("all", if all { "1" } else { "0" }),
            ("host", host.as_str()),
        ],
    )?;
    Ok(format!("{path}?{}", query.query().unwrap_or("")))
}
