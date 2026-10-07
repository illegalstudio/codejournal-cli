use clap::{Args, Subcommand, ValueEnum};

#[derive(Args)]
pub struct GardenArgs {
    /// Preview current findings without changing the journal.
    #[arg(long)]
    pub dry_run: bool,
    /// Show all pending findings, explicitly opting into the complete output.
    #[arg(long)]
    pub all: bool,
    #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u32).range(1..=25))]
    pub limit: u32,
    /// Continue an existing scan after this finding UUID.
    #[arg(long, conflicts_with = "dry_run", value_parser = continuation)]
    pub after: Option<String>,
    #[command(subcommand)]
    pub action: Option<GardenAction>,
}

#[derive(Subcommand)]
pub enum GardenAction {
    /// Record the result after verifying a finding or fixing its underlying content.
    Review(ReviewArgs),
}

#[derive(Args)]
pub struct ReviewArgs {
    #[arg(value_parser = finding)]
    pub id: String,
    #[arg(long, value_enum)]
    pub outcome: Outcome,
    /// Evidence for the decision, or what changed and where.
    #[arg(long, value_parser = note)]
    pub note: String,
    /// Future timestamp for a deferred review, for example 2026-10-14T00:00:00Z.
    #[arg(long, required_if_eq("outcome", "deferred"))]
    pub until: Option<String>,
    #[arg(long)]
    pub agent: Option<String>,
}

fn continuation(value: &str) -> Result<String, String> {
    uuid::Uuid::parse_str(value)
        .map(|id| id.to_string())
        .map_err(|_| "Use the finding UUID printed after Continue".into())
}

fn finding(value: &str) -> Result<String, String> {
    let hex = value.replace('-', "");
    if !(8..=32).contains(&hex.len()) || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(
            "Use a finding UUID or a prefix of at least eight hexadecimal characters".into(),
        );
    }
    Ok(value.to_ascii_lowercase())
}

fn note(value: &str) -> Result<String, String> {
    if value.trim().is_empty() || value.chars().count() > 10000 {
        return Err("A review needs an evidence note of 1-10000 characters".into());
    }
    Ok(value.to_owned())
}

#[derive(Clone, ValueEnum)]
pub enum Outcome {
    Verified,
    Corrected,
    Superseded,
    Obsolete,
    Deferred,
    Dismissed,
}

impl Outcome {
    pub fn name(&self) -> &str {
        match self {
            Self::Verified => "verified",
            Self::Corrected => "corrected",
            Self::Superseded => "superseded",
            Self::Obsolete => "obsolete",
            Self::Deferred => "deferred",
            Self::Dismissed => "dismissed",
        }
    }
}
