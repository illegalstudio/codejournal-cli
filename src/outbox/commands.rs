use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum HookAction {
    /// Inspect hook metadata without authentication. Bodies may contain private data.
    List {
        #[arg(long, help = "Include redacted event bodies")]
        body: bool,
    },
    /// Associate one unbound event with the current account, without sending it.
    Bind {
        id: String,
        #[arg(
            long,
            required = true,
            help = "Confirm this event belongs to the configured account"
        )]
        to_current_account: bool,
    },
    /// Permanently discard one queued hook event by its unique ID prefix.
    Drop { id: String },
}

pub fn run(action: HookAction, json_mode: bool, server: Option<&str>) -> Result<()> {
    match action {
        HookAction::List { body } => super::inventory::list(body, json_mode),
        HookAction::Bind { id, .. } => super::mutation::bind(&id, server, json_mode),
        HookAction::Drop { id } => super::mutation::drop(&id, json_mode),
    }
}
