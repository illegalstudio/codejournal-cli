use crate::activity::ActivityAction;
use crate::cli::{Cli, Command};
use crate::*;
use anyhow::Result;

pub fn run(api: &api::Api, server: &str, tenant: &str, cli: Cli) -> Result<()> {
    let project = cli.project.as_deref();
    match cli.command {
        Command::Outbox {
            action: outbox_commands::OutboxAction::Receipt { id },
        } => {
            let value = request_ids::receipt(api, tenant, &id)?;
            output::emit(
                &value,
                &format!(
                    "Request {} created {} {}",
                    id,
                    value["resource_type"].as_str().unwrap_or("resource"),
                    value["resource_id"].as_str().unwrap_or("")
                ),
                cli.json,
            )
        }
        Command::Whoami => output::json(&api.get("/api/v1/me")?),
        Command::Sync => storage_status::sync(&api, cli.json),
        Command::HookFlush { after } => {
            if let Some(seconds) = after {
                std::thread::sleep(std::time::Duration::from_secs(seconds));
            }
            outbox_flush::run(&api, &tenant)?;
            Ok(())
        }
        Command::Brief(args) => brief::run(&api, &tenant, project, args, cli.json),
        Command::Browse(args) => browse::run(&api, &tenant, project, args),
        Command::Export(args) => export::run(&api, &tenant, project, args),
        Command::Import { file } => import::run(&api, &tenant, file, cli.json),
        Command::Open => open_dashboard::open(api, server, tenant, project, cli.json),
        Command::Web(args) => open_dashboard::web(api, server, tenant, project, args, cli.json),
        Command::Search(args) => entry_search::run(&api, &tenant, project, args, cli.json),
        Command::Recent(args) => entry_search::recent(&api, &tenant, project, args, cli.json),
        Command::Show { id, no_track } => knowledge::show(&api, &tenant, &id, no_track, cli.json),
        Command::Topics { action } => topics::run(&api, &tenant, project, action, cli.json),
        Command::Supersede { id, by } => {
            knowledge_mutations::supersede(&api, &tenant, &id, &by, cli.json)
        }
        Command::Obsolete { id } => knowledge_mutations::obsolete(&api, &tenant, &id, cli.json),
        Command::Answer(args) => {
            let body = input::body(args.body, args.body_file)?;
            knowledge_mutations::answer(
                &api,
                &tenant,
                &args.id,
                &args.title,
                &args.kind,
                &body,
                args.refs,
                args.agent,
                cli.json,
            )
        }
        Command::Entry { action } => knowledge_mutations::entry(&api, &tenant, action, cli.json),
        Command::Garden(args) => crate::garden::run(&api, &tenant, project, args, cli.json),
        Command::Digest(args) => digest::run(
            &api,
            &tenant,
            project,
            &args.since,
            args.project_only,
            args.send,
            cli.json,
        ),
        Command::Add(args) => {
            let body = input::body(args.body, args.body_file)?;
            commands::add(
                &api,
                &tenant,
                project,
                &args.kind,
                &args.title,
                &body,
                args.topics,
                args.refs,
                args.agent,
                args.force,
                args.global_scope,
                cli.json,
            )
        }
        Command::Log { action } => logs::run(&api, &tenant, project, action, cli.json),
        Command::Task { action } => tasks::run(&api, &tenant, project, action, cli.json),
        Command::Feedback { action } => feedback::run(&api, &tenant, project, action, cli.json),
        Command::Project { action } => {
            project_commands::run(&api, &tenant, project, action, cli.json)
        }
        Command::Projects(args) => project_list::run(&api, &tenant, args, cli.json),
        Command::Checkouts { all_projects } => {
            project_paths::list(&api, &tenant, project, all_projects, cli.json)
        }
        Command::Normalize { dry_run } => normalize::run(&api, &tenant, dry_run, cli.json),
        Command::Refs { action } => refs_move::run(&api, &tenant, project, action, cli.json),
        Command::Plan { action } => plans::run(&api, &tenant, project, "plans", action, cli.json),
        Command::Doc { action } => plans::run(&api, &tenant, project, "docs", action, cli.json),
        Command::Rules(args) => rules::run(
            &api,
            &tenant,
            project,
            args.action,
            args.body,
            args.body_file,
            cli.json,
        ),
        Command::Activity {
            action: ActivityAction::Publish,
        } => activity::publish(&api, &tenant, project),
        Command::Watch { action } => {
            watches::run(&api, &server, &tenant, project, action, cli.json)
        }
        Command::WatchRun { id } => watches::run(
            &api,
            &server,
            &tenant,
            project,
            crate::watch_args::WatchAction::Run { id },
            cli.json,
        ),
        Command::Notifications { action } => {
            notifications::run(&api, &tenant, project, action, cli.json)
        }
        Command::Notify(args) => notification_send::run(&api, &tenant, project, args, cli.json),
        Command::Login
        | Command::Update(..)
        | Command::Setup(..)
        | Command::Logout
        | Command::Outbox { .. }
        | Command::Status
        | Command::Hook { .. }
        | Command::SessionRecord { .. }
        | Command::Hooks { .. } => {
            unreachable!()
        }
    }
}
