use clap::Subcommand;

#[derive(Subcommand)]
pub enum HooksAction {
    Install {
        #[arg(long, default_value = "all", value_parser = ["all", "codex", "claude", "opencode"])]
        agent: String,
        #[arg(long)]
        dry_run: bool,
    },
    Uninstall {
        #[arg(long, default_value = "all", value_parser = ["all", "codex", "claude", "opencode"])]
        agent: String,
        #[arg(long)]
        dry_run: bool,
    },
    Status {
        #[arg(long, default_value = "all", value_parser = ["all", "codex", "claude", "opencode"])]
        agent: String,
    },
}
