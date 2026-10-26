use clap::Args;

#[derive(Args)]
pub struct ContextArgs {
    pub id: String,
    #[arg(long, value_parser = ["host", "guest"])]
    pub role: Option<String>,
}

#[derive(Args)]
pub struct ReadArgs {
    #[command(flatten)]
    pub context: ContextArgs,
    #[arg(long)]
    pub after: Option<i64>,
    #[arg(skip)]
    pub full: bool,
}

#[derive(Args)]
pub struct WaitArgs {
    #[command(flatten)]
    pub read: ReadArgs,
    #[arg(long, default_value = "30", value_parser = wait_duration, help = "Wait budget in seconds, from 0 to 86400")]
    pub timeout: f64,
    #[arg(long, value_enum, default_value = "event")]
    pub until: Until,
}

#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Until {
    Event,
    Question,
    Result,
    Terminal,
}

pub fn duration(value: &str) -> Result<f64, String> {
    value
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite() && *n >= 0.0)
        .ok_or_else(|| "duration must be finite and nonnegative".to_owned())
}

fn wait_duration(value: &str) -> Result<f64, String> {
    duration(value).and_then(|n| {
        if n <= 86400.0 {
            Ok(n)
        } else {
            Err("wait timeout must be between 0 and 86400 seconds".to_owned())
        }
    })
}

#[derive(Args)]
pub struct BodyArgs {
    #[command(flatten)]
    pub context: ContextArgs,
    #[arg(long, conflicts_with = "body_file", allow_hyphen_values = true)]
    pub body: Option<String>,
    #[arg(long, conflicts_with = "body")]
    pub body_file: Option<String>,
    #[arg(long)]
    pub reply_to: Option<i64>,
    #[arg(long)]
    pub idempotency_key: Option<String>,
}

#[derive(Args)]
pub struct PublishArgs {
    #[command(flatten)]
    pub body: BodyArgs,
    #[arg(long, value_parser = ["ready", "status", "message", "question", "answer", "checkpoint", "result", "error"])]
    pub kind: String,
}
