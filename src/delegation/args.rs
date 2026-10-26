mod context_args;
mod start;
use clap::{Args, Subcommand};
pub use context_args::{BodyArgs, ContextArgs, PublishArgs, ReadArgs, Until, WaitArgs};
pub use start::{Mode, StartArgs, Target};

#[derive(Args)]
pub struct DelegationArgs {
    /// Include full task text, history and diagnostic event payloads.
    #[arg(long, global = true, conflicts_with = "id_only")]
    pub full: bool,
    /// Print only the UUID when starting or creating a delegation.
    #[arg(long, global = true)]
    pub id_only: bool,
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
    /// Wait for new events. Bare calls remember delivery separately from acknowledgements.
    Wait(WaitArgs),
    /// Wait for a final result or terminal failure, never return progress as a result.
    Result(WaitArgs),
    /// Read bounded, redacted provider diagnostics from this machine.
    Output {
        id: String,
        #[arg(long, default_value = "40", value_parser = clap::value_parser!(u16).range(1..=200))]
        lines: u16,
    },
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
        /// Replace the inactivity budget atomically with the new attempt (0 disables it).
        #[arg(long, value_parser = context_args::duration)]
        stall_timeout: Option<f64>,
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
