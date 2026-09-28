mod api;
mod commands;
mod config;
mod login;
mod output;
mod project;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about = "Code Journal device client")]
struct Cli {
    #[arg(long, global = true, env = "CJ_SERVER_URL")]
    server: Option<String>,
    #[arg(long, global = true, env = "CJ_PROJECT")]
    project: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Login,
    Logout,
    Whoami,
    Brief,
    Search {
        query: String,
    },
    Add {
        #[arg(long)]
        kind: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        body: String,
        #[arg(long, value_delimiter = ',')]
        topics: Vec<String>,
    },
    Log {
        #[arg(long)]
        title: String,
        #[arg(long)]
        body: String,
    },
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },
}

#[derive(Subcommand)]
enum TaskAction {
    List,
    Add {
        title: String,
        #[arg(long, default_value = "")]
        body: String,
        #[arg(long, default_value = "normal")]
        priority: String,
    },
}

#[derive(Subcommand)]
enum ProjectAction {
    List,
    Init {
        #[arg(long)]
        name: Option<String>,
    },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    if matches!(cli.command, Command::Login) {
        return login::run(cli.server.as_deref());
    }
    let mut config = config::Config::load()?;
    if matches!(cli.command, Command::Logout) {
        let api = api::Api::new(&config.server, &config.token()?)?;
        api.delete("/api/v1/device/token")?;
        return config.logout();
    }
    let server = cli.server.as_deref().unwrap_or(&config.server).to_owned();
    let token = config.token()?;
    let api = api::Api::new(&server, &token)?;
    let slug = config.tenant.clone();
    match cli.command {
        Command::Whoami => output::json(&api.get("/api/v1/me")?),
        Command::Brief => commands::brief(&api, &slug, cli.project.as_deref()),
        Command::Search { query } => commands::search(&api, &slug, cli.project.as_deref(), &query),
        Command::Add {
            kind,
            title,
            body,
            topics,
        } => commands::add(
            &api,
            &slug,
            cli.project.as_deref(),
            &kind,
            &title,
            &body,
            topics,
        ),
        Command::Log { title, body } => {
            commands::log(&api, &slug, cli.project.as_deref(), &title, &body)
        }
        Command::Task { action } => commands::task(&api, &slug, cli.project.as_deref(), action),
        Command::Project { action } => {
            commands::project(&api, &slug, cli.project.as_deref(), action)
        }
        Command::Login | Command::Logout => unreachable!(),
    }
}
