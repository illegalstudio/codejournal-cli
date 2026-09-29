use clap::{Args, Subcommand};

#[derive(Args)]
pub struct LogAddArgs {
    #[arg(long)]
    pub title: String,
    #[arg(long, default_value = "done")]
    pub status: String,
    #[arg(long)]
    pub plan: Option<String>,
    #[arg(long = "ref", help = crate::refs::HELP)]
    pub refs: Vec<String>,
    #[arg(long)]
    pub agent: Option<String>,
    #[arg(long, conflicts_with = "body_file")]
    pub body: Option<String>,
    #[arg(long)]
    pub body_file: Option<String>,
    #[arg(long)]
    pub no_auto_commits: bool,
}

#[derive(Args)]
pub struct LogListArgs {
    #[arg(long)]
    pub since: Option<String>,
    #[arg(long)]
    pub agent: Option<String>,
    #[arg(long)]
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
}
