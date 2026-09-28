use clap::Subcommand;

#[derive(Subcommand)]
pub enum PlanAction {
    List,
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
    },
    Show {
        id: String,
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
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        based_on: Option<u32>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        not_before: Option<String>,
    },
    Schedule {
        id: String,
        date: String,
    },
}
