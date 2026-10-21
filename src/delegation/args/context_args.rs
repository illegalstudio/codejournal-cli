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
    #[arg(long)]
    pub full: bool,
}

#[derive(Args)]
pub struct WaitArgs {
    #[command(flatten)]
    pub read: ReadArgs,
    #[arg(long, default_value = "30")]
    pub timeout: f64,
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
