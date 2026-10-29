use clap::{Args, Subcommand};

#[derive(Args)]
pub struct ReminderAdd {
    #[arg(long)]
    pub title: String,
    #[arg(long, help = "YYYY-MM-DD (09:00), local YYYY-MM-DDTHH:MM, or RFC3339")]
    pub at: String,
    #[arg(long, help = "IANA timezone, for example Europe/Rome")]
    pub timezone: String,
    #[arg(long, conflicts_with = "body_file", allow_hyphen_values = true)]
    pub body: Option<String>,
    #[arg(long)]
    pub body_file: Option<String>,
    #[arg(long, help = "Create a workspace reminder without a project")]
    pub global: bool,
    #[arg(
        long,
        conflicts_with = "global",
        help = "Announce only in the source project's brief"
    )]
    pub project_only: bool,
    #[arg(
        long,
        help = "Optional HTTP(S) link to the related task, plan, or other context"
    )]
    pub reference_url: Option<String>,
}

#[derive(Args)]
pub struct ReminderList {
    #[arg(long, default_value = "open", value_parser = ["open", "due", "upcoming", "closed", "all"])]
    pub state: String,
    #[arg(long, conflicts_with = "state")]
    pub all: bool,
    #[arg(
        long,
        help = "Limit to the current project; default covers the workspace"
    )]
    pub project_only: bool,
    #[arg(short, long, help = "Include reminder context and full metadata")]
    pub verbose: bool,
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..))]
    pub page: u32,
    #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u32).range(1..=100))]
    pub limit: u32,
}

#[derive(Subcommand)]
pub enum ReminderAction {
    Add(ReminderAdd),
    List(ReminderList),
    Show {
        id: String,
    },
    Close {
        id: String,
    },
    Reschedule {
        id: String,
        #[arg(long)]
        at: String,
        #[arg(long)]
        timezone: String,
    },
    /// Reserve an announcement for ten minutes. Requires an online response.
    Claim {
        id: String,
    },
    /// Record a user-facing announcement without closing the reminder.
    Presented {
        id: String,
        #[arg(long)]
        delivery_token: String,
    },
}
