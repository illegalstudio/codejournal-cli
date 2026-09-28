use crate::{output, project};
use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::json;

#[derive(Args)]
pub struct WebArgs {
    #[command(subcommand)]
    pub action: WebAction,
    #[arg(long, global = true, default_value_t = 8765)]
    pub port: u16,
    #[arg(long, global = true)]
    pub open: bool,
}

#[derive(Subcommand)]
pub enum WebAction {
    Start,
    Stop,
    Status,
    Run,
}

pub fn open(server: &str, tenant: &str, project_name: Option<&str>, json_mode: bool) -> Result<()> {
    let base = format!("{}/t/{tenant}/", server.trim_end_matches('/'));
    let url = if crate::git::root().is_some() {
        format!("{base}#/p/{}", project::slug(project_name)?)
    } else {
        base
    };
    if !json_mode {
        let _ = webbrowser::open(&url);
    }
    output::emit(&json!({"url": url}), &url, json_mode)
}

pub fn web(
    server: &str,
    tenant: &str,
    project_name: Option<&str>,
    args: WebArgs,
    json_mode: bool,
) -> Result<()> {
    match args.action {
        WebAction::Start | WebAction::Status => {
            if args.open {
                open(server, tenant, project_name, json_mode)
            } else {
                let url = format!("{}/t/{tenant}/", server.trim_end_matches('/'));
                output::emit(
                    &json!({"url": url, "hosted": true}),
                    &format!("Dashboard: {url}"),
                    json_mode,
                )
            }
        }
        WebAction::Stop => output::emit(
            &json!({"hosted": true, "stopped": false}),
            "The dashboard is served by Code Journal; no local server is running.",
            json_mode,
        ),
        WebAction::Run => open(server, tenant, project_name, json_mode),
    }
}
