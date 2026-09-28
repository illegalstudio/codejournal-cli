use clap::Subcommand;

#[derive(Subcommand)]
pub enum ProjectAction {
    List,
    Init {
        #[arg(long)]
        name: Option<String>,
    },
    Show,
    Edit {
        #[arg(long)]
        slug: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long, conflicts_with = "no_remote")]
        remote: Option<String>,
        #[arg(long)]
        no_remote: bool,
        #[arg(long)]
        from_git: bool,
        #[arg(long)]
        provides: Option<String>,
    },
    PathAdd {
        path: std::path::PathBuf,
    },
    PathRemove {
        path: std::path::PathBuf,
        #[arg(long)]
        host: Option<String>,
    },
    Merge {
        source: String,
        #[arg(long = "into")]
        target: String,
    },
}
