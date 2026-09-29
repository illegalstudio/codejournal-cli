use clap::Subcommand;

#[derive(Subcommand)]
pub enum PlanAction {
    List {
        #[arg(long, default_value = "open")]
        status: String,
        #[arg(long)]
        grep: Option<String>,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        all_projects: bool,
    },
    Create {
        #[arg(long)]
        title: String,
        #[arg(long, conflicts_with = "body_file")]
        body: Option<String>,
        #[arg(long)]
        body_file: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        not_before: Option<String>,
        #[arg(long = "ref", help = crate::refs::HELP)]
        refs: Vec<String>,
        #[arg(long)]
        agent: Option<String>,
    },
    Show {
        id: String,
        #[arg(long)]
        revision: Option<u32>,
        #[arg(long)]
        history: bool,
        #[arg(long)]
        body: bool,
    },
    Update {
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long, conflicts_with = "body_file")]
        body: Option<String>,
        #[arg(long)]
        body_file: Option<String>,
        #[arg(long = "ref", help = crate::refs::HELP)]
        refs: Vec<String>,
        #[arg(long)]
        note: Option<String>,
        #[arg(long = "base", alias = "based-on")]
        based_on: Option<u32>,
        #[arg(long)]
        agent: Option<String>,
    },
    Status {
        id: String,
        status: String,
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        agent: Option<String>,
    },
    Move {
        id: String,
        #[arg(long = "to")]
        target: String,
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        agent: Option<String>,
    },
    Schedule {
        id: String,
        date: String,
    },
}
