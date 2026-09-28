use crate::plan_args::PlanAction;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about = "Code Journal device client")]
pub struct Cli {
    #[arg(long, global = true, env = "CJ_SERVER_URL")]
    pub server: Option<String>,
    #[arg(long, global = true, env = "CJ_PROJECT")]
    pub project: Option<String>,
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
}

#[derive(Subcommand)]
pub enum TaskAction {
    List {
        #[arg(long)]
        all: bool,
    },
    Add {
        title: String,
        #[arg(long, default_value = "")]
        body: String,
        #[arg(long, default_value = "normal")]
        priority: String,
        #[arg(long = "ref")]
        refs: Vec<String>,
        #[arg(long)]
        not_before: Option<String>,
        #[arg(long = "to")]
        target: Option<String>,
        #[arg(long)]
        from_entry: Option<String>,
    },
    Edit {
        id: String,
        #[arg(long)]
        not_before: String,
    },
    Start {
        id: String,
    },
    Done {
        id: String,
        #[arg(long)]
        note: Option<String>,
    },
    Dismiss {
        id: String,
        #[arg(long)]
        note: Option<String>,
    },
    Reopen {
        id: String,
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
