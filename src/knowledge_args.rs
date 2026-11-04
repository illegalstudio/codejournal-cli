use clap::Subcommand;

#[derive(clap::Args)]
pub struct AnswerArgs {
    pub id: String,
    #[arg(long)]
    pub title: String,
    #[arg(long, default_value = "discovery")]
    pub kind: String,
    #[arg(long, conflicts_with = "body_file", allow_hyphen_values = true)]
    pub body: Option<String>,
    #[arg(long)]
    pub body_file: Option<String>,
    #[arg(long = "ref", help = crate::refs::HELP)]
    pub refs: Vec<String>,
    #[arg(long)]
    pub agent: Option<String>,
}

#[derive(Subcommand)]
pub enum TopicsAction {
    /// Find reusable topic names without retrieving the full topic inventory.
    Find {
        query: String,
        #[arg(long)]
        all_projects: bool,
    },
    Merge {
        sources: Vec<String>,
        #[arg(long)]
        into: String,
    },
    Similar {
        /// Analyze every project's topics explicitly.
        #[arg(long)]
        all_projects: bool,
    },
}

#[derive(Subcommand)]
pub enum EntryAction {
    /// Correct reference metadata without replacing the knowledge entry.
    Refs {
        id: String,
        #[arg(long, help = "Expected refs_version from cj entry show ID --json")]
        base: String,
        #[arg(long = "ref", conflicts_with = "clear_refs", required_unless_present = "clear_refs", help = crate::refs::UPDATE_HELP)]
        refs: Vec<String>,
        #[arg(long)]
        clear_refs: bool,
    },
    /// Read an entry (alias of cj show).
    Show {
        id: String,
        #[arg(long)]
        no_track: bool,
    },
    /// Transfer one entry while preserving its ID, timestamps, topics and usage history.
    Move {
        id: String,
        #[arg(long)]
        to: String,
        #[arg(long, requires = "replace_prefix")]
        path_prefix: Option<String>,
        #[arg(long, requires = "path_prefix")]
        replace_prefix: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    Flag {
        id: String,
        #[arg(long, conflicts_with = "helpful", required_unless_present = "helpful")]
        wrong: bool,
        #[arg(long, conflicts_with = "wrong", required_unless_present = "wrong")]
        helpful: bool,
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        agent: Option<String>,
    },
    Scope {
        id: String,
        scope: String,
    },
    SetAgent {
        id: String,
        agent: String,
    },
}
