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
    if let Command::RequestSync { origin } = &cli.command {
        return request_sync::worker::run(cli.server.as_deref(), origin);
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
        return storage_status::status(cli.json, cli.offline);
    }
    if matches!(&cli.command, Command::Outbox { action } if !matches!(action, outbox_commands::OutboxAction::Receipt { .. }))
        && let Command::Outbox { action } = cli.command
    {
        return outbox_commands::run(action, cli.json);
    }
    plan_validation::preflight(&mut cli.command)?;
    watch_validation::preflight(&cli.command)?;
    let mut config = config::Config::load()?;
    if matches!(cli.command, Command::Logout) {
        let api = api::Api::new(&config.server, &config.token()?)?;
        api.delete("/api/v1/device/token")?;
        return config.logout();
    }
    let server = cli.server.as_deref().unwrap_or(&config.server).to_owned();
    let token = match config.token_for(&server) {
        Ok(token) => token,
        Err(_)
            if cli.offline
                && server.trim_end_matches('/') == config.server.trim_end_matches('/') =>
        {
            eprintln!(
                "Offline without keyring access: writes can queue; credential-scoped cached reads are unavailable."
            );
            String::new()
        }
        Err(error) => return Err(error),
    };
    let mut api = api::Api::new(&server, &token)?;
    api.set_offline(cli.offline);
    if cli.project.is_none()
        && !matches!(
            &cli.command,
            Command::Feedback {
                action: feedback_args::FeedbackAction::Show { .. }
            }
        )
        && checkout_identity::current().is_none()
        && let project_folder::Scope::Named(slug) = project_folder::registered(&api, &config.tenant)
    {
        cli.project = Some(slug);
    }
    api.set_auto_project(cli.project.is_none() && checkout_identity::current().is_some());
    let tenant = config.tenant.clone();
    let json_mode = cli.json;
    let automatic = !cli.offline && !matches!(cli.command, Command::Sync);
    let result = dispatch::run(&api, &server, &tenant, cli);
    if automatic && let Err(error) = request_sync::wake(&api, &tenant) {
        eprintln!("Automatic synchronization could not start: {error}; run cj sync.");
    }
    match result {
        Err(error)
            if error
                .downcast_ref::<request_outbox::QueuedWrite>()
                .is_some() =>
        {
            let queued = error.downcast_ref::<request_outbox::QueuedWrite>().unwrap();
            if json_mode {
                let mut receipt = serde_json::json!({"queued": true, "id": queued.0, "request_id": queued.0, "resource_id": null, "next": "Background delivery retries when online. Run cj sync to retry now, then find the created resource with its list/search command. Do not use request_id as a resource ID."});
                if let Some(upgrade) = api_upgrade::details() {
                    receipt["upgrade_required"] = serde_json::Value::Bool(true);
                    receipt["compatibility"] = upgrade;
                }
                output::json(&receipt)?;
            } else {
                if let Some(notice) = output::masking_notice() {
                    crate::stdout::println!("{notice}");
                }
                crate::stdout::println!(
                    "Queued request_id: {} (not a resource ID). Background delivery retries when online. Run cj sync to retry now, then find the created resource with its list/search command.",
                    queued.0
                );
            }
            Ok(())
        }
        result => result,
    }
}
