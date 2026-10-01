use clap::{Args, Subcommand};

#[derive(Args)]
pub struct TaskAddArgs {
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long = "to")]
    pub target: Option<String>,
    #[arg(long)]
    pub from_entry: Option<String>,
    #[arg(long, default_value = "normal")]
    pub priority: String,
    #[arg(long)]
    pub plan: Option<String>,
    #[arg(long)]
    pub not_before: Option<String>,
    #[arg(long = "ref", help = crate::refs::HELP)]
    pub refs: Vec<String>,
    #[arg(long)]
    pub agent: Option<String>,
    #[arg(long, conflicts_with = "body_file", allow_hyphen_values = true)]
    pub body: Option<String>,
    #[arg(long)]
    pub body_file: Option<String>,
}

#[derive(Args)]
pub struct TaskListArgs {
    #[arg(long, default_value = "active")]
    pub status: String,
    #[arg(long)]
    pub all_projects: bool,
    #[arg(long = "from")]
    pub source: Option<String>,
    #[arg(long)]
    pub forwarded: bool,
    #[arg(long)]
    pub priority: Option<String>,
}

#[derive(Args)]
pub struct TaskEditArgs {
    pub id: String,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub priority: Option<String>,
    #[arg(long)]
    pub not_before: Option<String>,
    #[arg(long, conflicts_with = "no_plan")]
    pub plan: Option<String>,
    #[arg(long, conflicts_with = "plan")]
    pub no_plan: bool,
    #[arg(long)]
    pub agent: Option<String>,
}

#[derive(Subcommand)]
pub enum TaskAction {
    Add(TaskAddArgs),
    List(TaskListArgs),
    Show {
        id: String,
    },
    Start {
        id: String,
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        agent: Option<String>,
    },
    Done {
        id: String,
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        agent: Option<String>,
    },
    Dismiss {
        id: String,
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        agent: Option<String>,
    },
    Reopen {
        id: String,
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        agent: Option<String>,
    },
    Note {
        id: String,
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        body_file: Option<String>,
        #[arg(long)]
        agent: Option<String>,
    },
    Link {
        id: String,
        #[arg(required = true)]
        refs: Vec<String>,
        #[arg(long)]
        agent: Option<String>,
    },
    Unlink {
        id: String,
        #[arg(required = true)]
        refs: Vec<String>,
        #[arg(long)]
        agent: Option<String>,
    },
    Edit(TaskEditArgs),
}
