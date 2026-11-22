use clap::Args;

#[derive(Args)]
pub struct AddArgs {
    #[arg(long, value_parser = ["discovery", "architecture", "decision", "gotcha", "howto", "environment", "question"])]
    pub kind: String,
    #[arg(long)]
    pub title: String,
    #[arg(long, conflicts_with = "body_file", allow_hyphen_values = true)]
    pub body: Option<String>,
    #[arg(long)]
    pub body_file: Option<String>,
    #[arg(long, value_delimiter = ',')]
    pub topics: Vec<String>,
    #[arg(long = "ref", help = crate::refs::HELP)]
    pub refs: Vec<String>,
    #[arg(long)]
    pub agent: Option<String>,
    #[arg(
        long,
        help = "Allow creation despite an existing active entry with the same title; to replace a fact, use --supersedes ID instead"
    )]
    pub force: bool,
    /// Replace a fact in one step: record this entry and mark entry ID as superseded by it. The title may stay the same.
    #[arg(long, value_name = "ID")]
    pub supersedes: Option<String>,
    #[arg(long = "global")]
    pub global_scope: bool,
    /// Expire a project observation (kind discovery, gotcha or howto) after 30 days unless promoted; other kinds are refused.
    #[arg(long, conflicts_with = "global_scope")]
    pub temporary: bool,
    /// Keep an agent observation without a temporary deadline.
    #[arg(long, conflicts_with = "temporary")]
    pub durable: bool,
}
