use clap::Subcommand;

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
    List,
}
