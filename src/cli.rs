use crate::activity::ActivityAction;
use crate::plan_args::PlanAction;
use crate::project_args::ProjectAction;
use crate::task_args::TaskAction;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cj", version, about = "Code Journal device client")]
pub struct Cli {
    #[arg(long, global = true)]
    pub cwd: Option<std::path::PathBuf>,
    #[arg(long, global = true, env = "CJ_SERVER_URL")]
    pub server: Option<String>,
    #[arg(long, global = true, env = "CJ_PROJECT")]
    pub project: Option<String>,
    #[arg(long, global = true)]
    pub json: bool,
    #[arg(long, global = true)]
    pub offline: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Login,
    /// Update the installed binary and refresh configured agent skills.
    Update(crate::distribution::UpdateArgs),
    Setup(crate::setup_agents::SetupArgs),
    Logout,
    Whoami,
    Status,
    Sync,
    #[command(hide = true)]
    RequestSync {
        origin: String,
    },
    Outbox {
        #[command(subcommand)]
        action: crate::outbox_commands::OutboxAction,
    },
    Brief(crate::brief::BriefArgs),
    Browse(crate::browse::BrowseArgs),
    Export(crate::export::ExportArgs),
    Import {
        file: std::path::PathBuf,
    },
    Open,
    Web(crate::open_dashboard::WebArgs),
    Search(crate::search_args::SearchArgs),
    Recent(crate::search_args::RecentArgs),
    /// Read a knowledge entry. For work logs, use cj log show ID.
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
    Add(crate::add_args::AddArgs),
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
    Projects(crate::project_args::ProjectListArgs),
    Checkouts {
        #[arg(long)]
        all_projects: bool,
    },
    Normalize {
        #[arg(long)]
        dry_run: bool,
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
    Rules(crate::rules::RulesArgs),
    Activity {
        #[command(subcommand)]
        action: ActivityAction,
    },
    Watch {
        #[command(subcommand)]
        action: crate::watch_args::WatchAction,
    },
    WatchRun {
        id: String,
    },
    Notifications {
        #[command(subcommand)]
        action: crate::watch_args::NotificationAction,
    },
    Notify(crate::watch_args::NotifyArgs),
    Hook {
        event: String,
        kind: Option<String>,
    },
    SessionRecord {
        operation: String,
    },
    HookFlush {
        #[arg(long)]
        after: Option<u64>,
    },
    Hooks {
        #[command(subcommand)]
        action: crate::hook_args::HooksAction,
    },
    Garden(crate::garden::args::GardenArgs),
    Digest(crate::digest::DigestArgs),
}

#[cfg(test)]
#[path = "cli_body_tests.rs"]
mod body_tests;
