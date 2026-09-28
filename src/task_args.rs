use clap::Subcommand;

#[derive(Subcommand)]
pub enum TaskAction {
    List {
        #[arg(long)]
        all: bool,
    },
    Add {
        title: String,
        #[arg(long, default_value = "")]
        body: String,
        #[arg(long, default_value = "normal")]
        priority: String,
        #[arg(long = "ref")]
        refs: Vec<String>,
        #[arg(long)]
        not_before: Option<String>,
        #[arg(long = "to")]
        target: Option<String>,
        #[arg(long)]
        from_entry: Option<String>,
    },
    Edit {
        id: String,
        #[arg(long)]
        not_before: String,
    },
    Start {
        id: String,
    },
    Done {
        id: String,
        #[arg(long)]
        note: Option<String>,
    },
    Dismiss {
        id: String,
        #[arg(long)]
        note: Option<String>,
    },
    Reopen {
        id: String,
    },
}
