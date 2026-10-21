mod context_args;
mod start;
use clap::{Args, Subcommand};
pub use context_args::{BodyArgs, ContextArgs, PublishArgs, ReadArgs, WaitArgs};
pub use start::{Mode, StartArgs, Target};

#[derive(Args)]
pub struct DelegationArgs {
    #[command(subcommand)]
    pub action: Action,
}

#[derive(Subcommand)]
pub enum Action {
    /// Start a durable delegation using an installed agent CLI.
    Start(StartArgs),
    /// Run an agent synchronously without a managed session.
    Run(StartArgs),
    /// Create durable state without launching a guest.
    Create(StartArgs),
    Sync(ReadArgs),
    Status(ReadArgs),
    Wait(WaitArgs),
    Result(WaitArgs),
    Ack {
        #[command(flatten)]
        context: ContextArgs,
        #[arg(long)]
        through: i64,
    },
    Publish(PublishArgs),
    Send(BodyArgs),
    Answer(BodyArgs),
    Checkpoint(BodyArgs),
    List {
        #[arg(long, conflicts_with = "all")]
        active: bool,
        #[arg(long, conflicts_with = "active")]
        all: bool,
        #[arg(long, default_value = "50", value_parser = clap::value_parser!(u16).range(1..=500))]
        limit: u16,
    },
    Recover {
        id: String,
    },
    Integrate {
        id: String,
    },
    Cancel {
        id: String,
        #[arg(long, default_value = "cancelled by host")]
        reason: String,
    },
    Purge {
        id: String,
        #[arg(long)]
        force: bool,
    },
    Gc {
        #[arg(long, default_value = "7")]
        days: f64,
    },
    Models {
        target: Target,
    },
    /// Read the embedded host or guest protocol.
    Guide {
        #[arg(long)]
        guest: bool,
    },
    /// Serve delegation tools over MCP stdio.
    Mcp,
    McpConfig {
        id: String,
    },
    /// Import a stopped legacy broker database without changing the original.
    Migrate {
        #[arg(long = "from")]
        path: std::path::PathBuf,
        /// Merge disjoint history, translating its event cursors when the store is not empty.
        #[arg(long)]
        merge: bool,
    },
    #[command(hide = true, name = "_worker")]
    Worker {
        id: String,
    },
}
