use clap::Args;

#[derive(Args)]
pub struct TaskEditArgs {
    pub id: String,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long, value_parser = ["low", "normal", "high", "critical"])]
    pub priority: Option<String>,
    #[arg(long)]
    pub not_before: Option<String>,
    #[arg(long, conflicts_with = "no_plan")]
    pub plan: Option<String>,
    #[arg(long, conflicts_with = "plan")]
    pub no_plan: bool,
    #[arg(
        long,
        conflicts_with = "body_file",
        allow_hyphen_values = true,
        help = "Replace the body; requires --base BODY_VERSION from cj task show ID --json"
    )]
    pub body: Option<String>,
    #[arg(
        long,
        help = "Replace the body from a file; requires --base from cj task show ID --json"
    )]
    pub body_file: Option<String>,
    #[arg(long, conflicts_with_all = ["body", "body_file"], allow_hyphen_values = true,
        help = "Append context without replacing the body; requires --base from cj task show ID --json")]
    pub append_body: Option<String>,
    #[arg(long = "ref", conflicts_with = "clear_refs", help = crate::refs::UPDATE_HELP)]
    pub refs: Vec<String>,
    #[arg(long)]
    pub clear_refs: bool,
    #[arg(
        long,
        help = "Expected body_version from cj task show; required for body edits",
        value_parser = body_version
    )]
    pub base: Option<String>,
    #[arg(long)]
    pub agent: Option<String>,
}

fn body_version(value: &str) -> Result<String, String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("use the 64-character body_version returned by cj task show".to_owned());
    }
    Ok(value.to_ascii_lowercase())
}
