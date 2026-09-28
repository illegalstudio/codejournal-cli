use clap::{Args, Subcommand};

#[derive(Subcommand)]
pub enum WatchAction {
    Start {
        #[arg(long)]
        title: String,
        #[arg(long, default_value = "all", value_parser = ["all", "failure"])]
        notify_on: String,
        #[arg(long)]
        timeout: Option<u64>,
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },
    List {
        #[arg(long)]
        all: bool,
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
    #[arg(long, conflicts_with = "body_file")]
    pub body: Option<String>,
    #[arg(long)]
    pub body_file: Option<String>,
}
