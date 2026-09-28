use crate::api::Api;
use crate::watch_args::NotificationAction;
use crate::{output, project, watches};
use anyhow::Result;

pub fn run(
    api: &Api,
    tenant: &str,
    project_name: Option<&str>,
    action: NotificationAction,
) -> Result<()> {
    let project = project::slug(project_name)?;
    match action {
        NotificationAction::List => output::json(&api.get(&format!(
            "{}/notifications",
            watches::endpoint(tenant, &project)
        ))?),
    }
}
