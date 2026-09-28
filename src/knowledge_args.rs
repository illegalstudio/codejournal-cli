use clap::Subcommand;

#[derive(clap::Args)]
pub struct AnswerArgs {
    pub id: String,
    #[arg(long)]
    pub title: String,
    #[arg(long, default_value = "discovery")]
    pub kind: String,
    #[arg(long, conflicts_with = "body_file")]
    pub body: Option<String>,
    #[arg(long)]
    pub body_file: Option<String>,
    #[arg(long = "ref")]
    pub refs: Vec<String>,
    #[arg(long)]
    pub agent: Option<String>,
}

#[derive(Subcommand)]
pub enum TopicsAction {
    Merge {
        sources: Vec<String>,
        #[arg(long)]
        into: String,
    },
    Similar,
}

#[derive(Subcommand)]
pub enum EntryAction {
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
