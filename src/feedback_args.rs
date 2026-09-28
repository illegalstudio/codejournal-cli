use clap::{Args, Subcommand};

#[derive(Args)]
pub struct FeedbackAddArgs {
    #[arg(long)]
    pub category: String,
    #[arg(long)]
    pub title: String,
    #[arg(long)]
    pub agent: Option<String>,
    #[arg(long, conflicts_with = "body_file")]
    pub body: Option<String>,
    #[arg(long)]
    pub body_file: Option<String>,
    #[arg(long)]
    pub force: bool,
}

#[derive(Args)]
pub struct FeedbackListArgs {
    #[arg(long, default_value = "open")]
    pub status: String,
    #[arg(long)]
    pub category: Option<String>,
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum FeedbackAction {
    Add(FeedbackAddArgs),
    List(FeedbackListArgs),
    Close {
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
