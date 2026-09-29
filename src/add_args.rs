use clap::Args;

#[derive(Args)]
pub struct AddArgs {
    #[arg(long)]
    pub kind: String,
    #[arg(long)]
    pub title: String,
    #[arg(long, conflicts_with = "body_file")]
    pub body: Option<String>,
    #[arg(long)]
    pub body_file: Option<String>,
    #[arg(long, value_delimiter = ',')]
    pub topics: Vec<String>,
    #[arg(long = "ref", help = crate::refs::HELP)]
    pub refs: Vec<String>,
    #[arg(long)]
    pub agent: Option<String>,
    #[arg(long)]
    pub force: bool,
    #[arg(long = "global")]
    pub global_scope: bool,
}
