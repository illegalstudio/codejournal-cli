mod activity;
mod api;
mod cli;
mod commands;
mod config;
mod git;
mod input;
mod login;
mod output;
mod path_ref;
mod plan_args;
mod plans;
mod project;
mod refs;
mod rules;
mod shorthand;
mod tasks;

use anyhow::Result;
use clap::Parser;
use cli::{ActivityAction, Cli, Command};

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    if matches!(cli.command, Command::Login) {
        return login::run(cli.server.as_deref());
    }
    let mut config = config::Config::load()?;
    if matches!(cli.command, Command::Logout) {
        let api = api::Api::new(&config.server, &config.token()?)?;
        api.delete("/api/v1/device/token")?;
        return config.logout();
    }
    let server = cli.server.as_deref().unwrap_or(&config.server).to_owned();
    let api = api::Api::new(&server, &config.token()?)?;
    let tenant = config.tenant.clone();
    let project = cli.project.as_deref();
    match cli.command {
        Command::Whoami => output::json(&api.get("/api/v1/me")?),
        Command::Brief => commands::brief(&api, &tenant, project),
        Command::Search { query } => commands::search(&api, &tenant, project, &query),
        Command::Add {
            kind,
            title,
            body,
            body_file,
            topics,
            refs,
        } => {
            let body = input::body(body, body_file)?;
            commands::add(&api, &tenant, project, &kind, &title, &body, topics, refs)
        }
        Command::Log {
            title,
            body,
            body_file,
            refs,
        } => {
            let body = input::body(body, body_file)?;
            commands::log(&api, &tenant, project, &title, &body, refs)
        }
        Command::Task { action } => tasks::run(&api, &tenant, project, action),
        Command::Project { action } => commands::project(&api, &tenant, project, action),
        Command::Plan { action } => plans::run(&api, &tenant, project, "plans", action),
        Command::Doc { action } => plans::run(&api, &tenant, project, "docs", action),
        Command::Rules { action } => rules::run(&api, &tenant, project, action),
        Command::Activity {
            action: ActivityAction::Publish,
        } => activity::publish(&api, &tenant, project),
        Command::Login | Command::Logout => unreachable!(),
    }
}
