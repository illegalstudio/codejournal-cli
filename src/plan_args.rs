use clap::Subcommand;

#[derive(Subcommand)]
pub enum PlanAction {
    List {
        #[arg(
            long,
            help = "Select a status explicitly; defaults to current docs or active plans"
        )]
        status: Option<String>,
        #[arg(
            long,
            conflicts_with = "status",
            help = "Include every lifecycle status"
        )]
        all: bool,
        #[arg(short, long, help = "Include bodies and full record metadata")]
        verbose: bool,
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
        #[arg(
            long,
            conflicts_with = "body_file",
            allow_hyphen_values = true,
            help = "Markdown body (maximum 100000 characters)"
        )]
        body: Option<String>,
        #[arg(
            long,
            help = "Read Markdown from a file or - for stdin (maximum 100000 characters)"
        )]
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
        #[arg(long, conflicts_with_all = ["history", "all_revisions"], value_parser = clap::value_parser!(u32).range(1..=2147483647), help = "Fetch only this numbered revision, without history")]
        revision: Option<u32>,
        #[arg(
            long,
            help = "List a page of revision metadata without historical bodies"
        )]
        history: bool,
        #[arg(long, conflicts_with_all = ["history", "before_revision", "revision", "current_only", "body"], help = "Fetch every revision and its content within the server response budget; use --history for larger histories")]
        all_revisions: bool,
        #[arg(long, requires = "history", conflicts_with = "current_only", value_parser = clap::value_parser!(u32).range(1..=2147483647))]
        before_revision: Option<u32>,
        #[arg(long, conflicts_with_all = ["history", "revision", "all_revisions"], help = "Explicitly select the default: current version only, including JSON")]
        current_only: bool,
        #[arg(long, conflicts_with_all = ["history", "all_revisions"], help = "Print only the current or selected revision body")]
        body: bool,
    },
    Update {
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(
            long,
            conflicts_with = "body_file",
            allow_hyphen_values = true,
            help = "Markdown body (maximum 100000 characters)"
        )]
        body: Option<String>,
        #[arg(
            long,
            help = "Read Markdown from a file or - for stdin (maximum 100000 characters)"
        )]
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
        #[arg(
            long,
            alias = "include-linked-logs",
            help = "Also transfer linked live work logs (plans only)"
        )]
        with_logs: bool,
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
