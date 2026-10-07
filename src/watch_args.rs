use clap::{Args, Subcommand};

#[derive(Subcommand)]
pub enum WatchAction {
    Start {
        #[arg(long)]
        title: String,
        #[arg(long, default_value = "end", value_parser = ["end", "failure"])]
        notify_on: String,
        /// Maximum runtime: seconds or a duration such as 30s, 10m, 3h or 1d (1-86400 seconds).
        #[arg(long, value_parser = crate::watch_validation::timeout)]
        timeout: Option<u64>,
        #[arg(long)]
        agent: Option<String>,
        /// Command and arguments: at most 100 items, each at most 2000 characters. Use a script file for longer code.
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },
    List {
        #[arg(long)]
        all: bool,
        #[arg(short, long, help = "Include commands and full record metadata")]
        verbose: bool,
    },
    Cancel {
        id: String,
    },
    #[command(hide = true)]
    Run {
        id: String,
    },
}

#[derive(Subcommand)]
pub enum NotificationAction {
    List {
        #[arg(long, conflicts_with = "read")]
        unread: bool,
        #[arg(long)]
        read: bool,
        #[arg(long, conflicts_with_all = ["unread", "read"], help = "Include read notifications")]
        all: bool,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        project_only: bool,
        #[arg(long, default_value_t = 50)]
        limit: u32,
        #[arg(short, long)]
        verbose: bool,
    },
    Read {
        ids: Vec<String>,
        #[arg(long)]
        all: bool,
    },
    Unread {
        ids: Vec<String>,
        #[arg(long)]
        all: bool,
    },
    Config {
        #[arg(long, value_parser = ["on", "off"])]
        desktop: Option<String>,
        #[arg(long)]
        ntfy: Option<String>,
    },
}

#[derive(Args)]
pub struct NotifyArgs {
    #[arg(long, default_value = "needs_input")]
    pub kind: String,
    #[arg(long)]
    pub title: String,
    #[arg(long)]
    pub agent: Option<String>,
    #[arg(long, conflicts_with = "body_file", allow_hyphen_values = true)]
    pub body: Option<String>,
    #[arg(long)]
    pub body_file: Option<String>,
}
