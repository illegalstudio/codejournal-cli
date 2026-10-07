use clap::Args;

#[derive(Args)]
pub struct SearchArgs {
    pub query: Option<String>,
    #[arg(long)]
    pub kind: Option<String>,
    #[arg(long)]
    pub topic: Option<String>,
    #[arg(long)]
    pub path: Option<String>,
    #[arg(long)]
    pub any: bool,
    #[arg(long)]
    pub prefix: bool,
    #[arg(long, alias = "all")]
    pub all_statuses: bool,
    #[arg(short, long, help = "Include entry bodies and full record metadata")]
    pub verbose: bool,
    #[arg(long)]
    pub all_projects: bool,
    #[arg(long, default_value_t = 20)]
    pub limit: u32,
}

#[derive(Args)]
pub struct RecentArgs {
    #[arg(long, default_value_t = 20)]
    pub limit: u32,
    #[arg(long)]
    pub kind: Option<String>,
    #[arg(long, alias = "all-statuses")]
    pub all: bool,
    #[arg(short, long)]
    pub verbose: bool,
}
