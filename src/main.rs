mod activity;
mod api;
mod attribution;
mod brief;
mod brief_format;
mod cli;
mod commands;
mod config;
mod dispatch;
mod entry_search;
mod feedback;
mod feedback_args;
mod git;
mod hook;
mod hook_args;
mod hook_settings;
mod hook_setup;
mod input;
mod knowledge;
mod knowledge_args;
mod knowledge_mutations;
mod log_args;
mod log_list;
mod login;
mod logs;
mod notifications;
mod output;
mod path_ref;
mod plan_args;
mod plan_changes;
mod plan_read;
mod plan_write;
mod plans;
mod project;
mod project_args;
mod project_commands;
mod project_list;
mod project_paths;
mod refs;
mod refs_move;
mod rules;
mod search_args;
mod session_git;
mod session_state;
mod shorthand;
mod staleness;
mod task_add;
mod task_args;
mod task_change;
mod task_read;
mod task_show;
mod tasks;
mod topic_similarity;
mod topic_stem;
mod topics;
mod watch_args;
mod watch_runner;
mod watch_state;
mod watches;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    if let Some(cwd) = &cli.cwd {
        std::env::set_current_dir(cwd)?;
    }
    if let Command::Hook { event } = &cli.command {
        return hook::run(event);
    }
    if let Command::Hooks { action } = &cli.command {
        return hook_setup::run(action);
    }
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
    dispatch::run(&api, &server, &tenant, cli)
}
