use clap::{Args, ValueEnum};

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Target {
    Claude,
    Codex,
    Cursor,
    Grok,
    Kimi,
    Pi,
}

impl Target {
    pub fn name(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Cursor => "cursor",
            Self::Grok => "grok",
            Self::Kimi => "kimi",
            Self::Pi => "pi",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, ValueEnum)]
pub enum Mode {
    #[default]
    ReadOnly,
    Write,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::Write => "write",
        }
    }
}

#[derive(Args)]
pub struct StartArgs {
    pub target: Target,
    #[arg(long, value_enum, default_value = "read-only")]
    pub mode: Mode,
    #[arg(long)]
    pub model: Option<String>,
    /// Task text, at most 100000 characters. Use context attachments for longer evidence.
    #[arg(long, conflicts_with = "prompt_file", allow_hyphen_values = true)]
    pub prompt: Option<String>,
    #[arg(long, conflicts_with = "prompt")]
    pub prompt_file: Option<String>,
    /// Snapshot a UTF-8 context file for bounded guest reads, repeatable.
    #[arg(long)]
    pub context_file: Vec<std::path::PathBuf>,
    /// Attach a previous terminal delegation result without granting cross-session access.
    #[arg(long)]
    pub context_result: Vec<String>,
    #[arg(long, default_value = "600")]
    pub stall_timeout: f64,
    #[arg(long, default_value = "300")]
    pub diff_timeout: f64,
    /// Inspect the provider command without launching it or creating state.
    #[arg(long)]
    pub dry_run: bool,
}
