use crate::{api::Api, attribution, project, project_paths};
use anyhow::{Result, ensure};
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Args)]
pub struct CheckoutArgs {
    #[arg(long)]
    all_projects: bool,
    #[command(subcommand)]
    action: Option<CheckoutAction>,
}

#[derive(Subcommand)]
enum CheckoutAction {
    /// Forget one exact recorded path without changing files on disk.
    Remove {
        path: PathBuf,
        #[arg(long, help = "Recorded host, defaults to this host")]
        host: Option<String>,
    },
}

pub fn run(
    api: &Api,
    tenant: &str,
    explicit: Option<&str>,
    args: CheckoutArgs,
    json_mode: bool,
) -> Result<()> {
    if let Some(CheckoutAction::Remove { path, host }) = args.action {
        ensure!(
            !args.all_projects,
            "select one project to remove a checkout; omit --all-projects"
        );
        return project_paths::remove(
            api,
            tenant,
            &project::slug(explicit)?,
            &path,
            host.unwrap_or_else(attribution::host),
            json_mode,
        );
    }
    project_paths::list(api, tenant, explicit, args.all_projects, json_mode)
}
