use crate::cli::Command;
use crate::*;
use anyhow::Result;

pub fn run() -> Result<()> {
    let mut cli = cli_parse::parse();
    if cli.project.is_some()
        && matches!(
            &cli.command,
            Command::Doc {
                action: plan_args::PlanAction::Create { global: true, .. }
            }
        )
    {
        anyhow::bail!("--global conflicts with --project when creating a doc");
    }
    if let Some(cwd) = &cli.cwd {
        std::env::set_current_dir(cwd)?;
    }
    if let Command::Hook { event, kind } = &cli.command {
        let _ = hook::run(event, kind.as_deref());
        return Ok(());
    }
    if let Command::SessionRecord { operation } = &cli.command {
        return session_record::run(operation);
    }
    if let Command::Hooks { action } = &cli.command {
        return hook_setup::run(action, cli.json);
    }
    if let Command::Setup(args) = &cli.command {
        if args.database.is_some() || args.sync_url.is_some() || args.auth_token.is_some() {
            anyhow::bail!(
                "Turso setup options are no longer used; run `cj login` for the hosted journal"
            );
        }
        return match &args.action {
            Some(action) => setup_agents::run(action),
            None => login::run(cli.server.as_deref()),
        };
    }
    if let Command::Update(args) = &cli.command {
        return distribution::run(args, cli.json);
    }
    if matches!(cli.command, Command::Login) {
        return login::run(cli.server.as_deref());
    }
    if matches!(cli.command, Command::Status) {
        return storage_status::status(cli.json);
    }
    if let Command::Outbox { action } = cli.command {
        return outbox_commands::run(action, cli.json);
    }
    let mut config = config::Config::load()?;
    if matches!(cli.command, Command::Logout) {
        let api = api::Api::new(&config.server, &config.token()?)?;
        api.delete("/api/v1/device/token")?;
        return config.logout();
    }
    let server = cli.server.as_deref().unwrap_or(&config.server).to_owned();
    let mut api = api::Api::new(&server, &config.token_for(&server)?)?;
    api.set_offline(cli.offline);
    if cli.project.is_none()
        && checkout_identity::current().is_none()
        && let project_folder::Scope::Named(slug) = project_folder::registered(&api, &config.tenant)
    {
        cli.project = Some(slug);
    }
    api.set_auto_project(cli.project.is_none() && checkout_identity::current().is_some());
    let tenant = config.tenant.clone();
    let json_mode = cli.json;
    match dispatch::run(&api, &server, &tenant, cli) {
        Err(error)
            if error
                .downcast_ref::<request_outbox::QueuedWrite>()
                .is_some() =>
        {
            let queued = error.downcast_ref::<request_outbox::QueuedWrite>().unwrap();
            if json_mode {
                crate::stdout::println!("{}", serde_json::json!({"queued": true, "id": queued.0}));
            } else {
                if let Some(notice) = output::masking_notice() {
                    crate::stdout::println!("{notice}");
                }
                crate::stdout::println!("Queued for synchronization: {}", queued.0);
            }
            Ok(())
        }
        result => result,
    }
}
