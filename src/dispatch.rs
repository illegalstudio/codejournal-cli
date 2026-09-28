use crate::cli::{ActivityAction, Cli, Command};
use crate::*;
use anyhow::Result;

pub fn run(api: &api::Api, server: &str, tenant: &str, cli: Cli) -> Result<()> {
    let project = cli.project.as_deref();
    match cli.command {
        Command::Whoami => output::json(&api.get("/api/v1/me")?),
        Command::Brief(args) => brief::run(&api, &tenant, project, args, cli.json),
        Command::Search(args) => entry_search::run(&api, &tenant, project, args, cli.json),
        Command::Recent { limit, kind } => {
            entry_search::recent(&api, &tenant, project, limit, kind.as_deref(), cli.json)
        }
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
        Command::Garden { dry_run } => commands::garden(&api, &tenant, project, dry_run),
        Command::Add {
            kind,
            title,
            body,
            body_file,
            topics,
            refs,
            agent,
            force,
            global_scope,
        } => {
            let body = input::body(body, body_file)?;
            commands::add(
                &api,
                &tenant,
                project,
                &kind,
                &title,
                &body,
                topics,
                refs,
                agent,
                force,
                global_scope,
                cli.json,
            )
        }
        Command::Log { action } => logs::run(&api, &tenant, project, action, cli.json),
        Command::Task { action } => tasks::run(&api, &tenant, project, action, cli.json),
        Command::Feedback { action } => feedback::run(&api, &tenant, project, action, cli.json),
        Command::Project { action } => {
            project_commands::run(&api, &tenant, project, action, cli.json)
        }
        Command::Projects => project_list::run(&api, &tenant, cli.json),
        Command::Checkouts { all_projects } => {
            project_paths::list(&api, &tenant, project, all_projects, cli.json)
        }
        Command::Refs { action } => refs_move::run(&api, &tenant, project, action, cli.json),
        Command::Plan { action } => plans::run(&api, &tenant, project, "plans", action, cli.json),
        Command::Doc { action } => plans::run(&api, &tenant, project, "docs", action, cli.json),
        Command::Rules { action } => rules::run(&api, &tenant, project, action),
        Command::Activity {
            action: ActivityAction::Publish,
        } => activity::publish(&api, &tenant, project),
        Command::Watch { action } => {
            watches::run(&api, &server, &tenant, project, action, cli.json)
        }
        Command::Notifications { action } => {
            notifications::run(&api, &tenant, project, action, cli.json)
        }
        Command::Notify(args) => notifications::notify(&api, &tenant, project, args, cli.json),
        Command::Login | Command::Logout | Command::Hook { .. } | Command::Hooks { .. } => {
            unreachable!()
        }
    }
}
