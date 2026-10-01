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
        #[arg(
            long,
            conflicts_with = "all_projects",
            help = "List tenant-global docs"
        )]
        global: bool,
        #[arg(
            long,
            conflicts_with_all = ["global", "all_projects"],
            help = "List only this project's docs, excluding global docs"
        )]
        local: bool,
    },
    Create {
        #[arg(long, conflicts_with = "project", help = "Create a tenant-global doc")]
        global: bool,
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
        #[arg(long, conflicts_with_all = ["history", "revision"], help = "Fetch only the current version, without revision history (also applies to JSON)")]
        current_only: bool,
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
        #[arg(long = "ref", help = crate::refs::UPDATE_HELP)]
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
        #[arg(
            long = "to",
            help = "Destination project slug; use @global to make a doc tenant-global"
        )]
        target: String,
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        agent: Option<String>,
    },
    /// Set one numbered Markdown checklist item, preserving other content.
    Step {
        id: String,
        #[arg(value_parser = clap::value_parser!(u32).range(1..))]
        number: u32,
        #[arg(long, required_unless_present = "undone", conflicts_with = "undone")]
        done: bool,
        #[arg(long)]
        undone: bool,
        #[arg(long)]
        note: Option<String>,
    },
    Schedule {
        id: String,
        date: String,
    },
}
