use clap::{Args, Subcommand};

#[derive(Args, Default)]
pub struct ProjectListArgs {
    /// List archived projects only.
    #[arg(long, conflicts_with = "all")]
    pub archived: bool,
    /// List active and archived projects.
    #[arg(long)]
    pub all: bool,
}

#[derive(Subcommand)]
pub enum ProjectAction {
    List(ProjectListArgs),
    /// Archive this project without deleting its history. Requires an online connection.
    Archive,
    /// Restore this archived project and allow writes again. Requires an online connection.
    Restore,
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
