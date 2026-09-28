use crate::plan_args::PlanAction;
use crate::project_args::ProjectAction;
use crate::task_args::TaskAction;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about = "Code Journal device client")]
pub struct Cli {
    #[arg(long, global = true)]
    pub cwd: Option<std::path::PathBuf>,
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
    Brief(crate::brief::BriefArgs),
    Search(crate::search_args::SearchArgs),
    Recent {
        #[arg(long, default_value_t = 20)]
        limit: u32,
        #[arg(long)]
        kind: Option<String>,
    },
    Show {
        id: String,
        #[arg(long)]
        no_track: bool,
    },
    Topics {
        #[command(subcommand)]
        action: Option<crate::knowledge_args::TopicsAction>,
    },
    Supersede {
        id: String,
        #[arg(long)]
        by: String,
    },
    Obsolete {
        id: String,
    },
    Answer(crate::knowledge_args::AnswerArgs),
    Entry {
        #[command(subcommand)]
        action: crate::knowledge_args::EntryAction,
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
        #[arg(long)]
        agent: Option<String>,
        #[arg(long)]
        force: bool,
        #[arg(long = "global")]
        global_scope: bool,
    },
    Log {
        #[command(subcommand)]
        action: crate::log_args::LogAction,
    },
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },
    Feedback {
        #[command(subcommand)]
        action: crate::feedback_args::FeedbackAction,
    },
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },
    Projects,
    Checkouts {
        #[arg(long)]
        all_projects: bool,
    },
    Refs {
        #[command(subcommand)]
        action: crate::refs_move::RefsAction,
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
    Notify(crate::watch_args::NotifyArgs),
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
pub enum ActivityAction {
    Publish,
}
