use crate::plan_args::PlanAction;
use crate::task_args::TaskAction;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about = "Code Journal device client")]
pub struct Cli {
    #[arg(long, global = true, env = "CJ_SERVER_URL")]
    pub server: Option<String>,
    #[arg(long, global = true, env = "CJ_PROJECT")]
    pub project: Option<String>,
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Login,
    Logout,
    Whoami,
    Brief,
    Search {
        query: String,
    },
    Add {
        #[arg(long)]
        kind: String,
        #[arg(long)]
        title: String,
        #[arg(long, conflicts_with = "body_file")]
        body: Option<String>,
        #[arg(long)]
        body_file: Option<String>,
        #[arg(long, value_delimiter = ',')]
        topics: Vec<String>,
        #[arg(long = "ref")]
        refs: Vec<String>,
    },
    Log {
        #[arg(long)]
        title: String,
        #[arg(long, conflicts_with = "body_file")]
        body: Option<String>,
        #[arg(long)]
        body_file: Option<String>,
        #[arg(long = "ref")]
        refs: Vec<String>,
        #[arg(long)]
        no_auto_commits: bool,
    },
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },
    Plan {
        #[command(subcommand)]
        action: PlanAction,
    },
    Doc {
        #[command(subcommand)]
        action: PlanAction,
    },
    Rules {
        #[command(subcommand)]
        action: Option<crate::rules::RulesAction>,
    },
    Activity {
        #[command(subcommand)]
        action: ActivityAction,
    },
    Watch {
        #[command(subcommand)]
        action: crate::watch_args::WatchAction,
    },
    Notifications {
        #[command(subcommand)]
        action: crate::watch_args::NotificationAction,
    },
    Hook {
        event: String,
    },
    Hooks {
        #[command(subcommand)]
        action: crate::hook_args::HooksAction,
    },
    Garden {
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
pub enum ProjectAction {
    List,
    Init {
        #[arg(long)]
        name: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ActivityAction {
    Publish,
}
