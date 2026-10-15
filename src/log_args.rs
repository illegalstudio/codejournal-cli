use clap::{Args, Subcommand};

#[derive(Args)]
pub struct LogAddArgs {
    #[arg(long)]
    pub title: String,
    #[arg(long, default_value = "done", value_parser = ["done", "in_progress", "blocked"])]
    pub status: String,
    #[arg(long)]
    pub plan: Option<String>,
    #[arg(long = "ref", help = crate::refs::HELP)]
    pub refs: Vec<String>,
    #[arg(long)]
    pub agent: Option<String>,
    #[arg(long, conflicts_with = "body_file", allow_hyphen_values = true)]
    pub body: Option<String>,
    #[arg(long)]
    pub body_file: Option<String>,
    #[arg(long)]
    pub no_auto_commits: bool,
}

#[derive(Args)]
pub struct LogListArgs {
    #[arg(long, value_parser = crate::since_arg::parse, help = crate::since_arg::HELP)]
    pub since: Option<String>,
    #[arg(long)]
    pub agent: Option<String>,
    #[arg(long, value_parser = ["done", "in_progress", "blocked", "all"])]
    pub status: Option<String>,
    #[arg(long)]
    pub plan: Option<String>,
    #[arg(long)]
    pub grep: Option<String>,
    #[arg(long)]
    pub all_projects: bool,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum LogAction {
    Add(LogAddArgs),
    List(LogListArgs),
    /// Amend a work log with revision protection, or replace its refs.
    Update(LogUpdateArgs),
    /// Read one work log by its full ID or unique prefix.
    Show {
        id: String,
        #[arg(long, help = "Print only the log body")]
        body: bool,
    },
    /// Transfer a work log while preserving its identity, content and plan link.
    Move {
        id: String,
        #[arg(long = "to")]
        target: String,
        #[arg(long, help = "Preview without changing or queueing the log")]
        dry_run: bool,
    },
}

#[derive(Args)]
#[command(group(clap::ArgGroup::new("changes").required(true).multiple(true).args(["refs", "clear_refs", "body", "body_file", "status"])))]
pub struct LogUpdateArgs {
    pub id: String,
    #[arg(long = "ref", conflicts_with = "clear_refs", help = crate::refs::UPDATE_HELP)]
    pub refs: Vec<String>,
    #[arg(long)]
    pub clear_refs: bool,
    #[arg(
        long,
        requires = "base",
        conflicts_with = "body_file",
        allow_hyphen_values = true
    )]
    pub body: Option<String>,
    #[arg(long, requires = "base")]
    pub body_file: Option<String>,
    #[arg(long, requires = "base", value_parser = ["done", "in_progress", "blocked"])]
    pub status: Option<String>,
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=2147483646), help = "Expected revision from cj log show; required for body/status edits")]
    pub base: Option<u32>,
}
