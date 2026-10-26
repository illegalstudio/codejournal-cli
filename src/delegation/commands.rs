use super::{
    args::{self, Action},
    context, store,
};
use anyhow::Result;
use serde_json::Value;

pub fn run(
    options: args::DelegationArgs,
    project: Option<String>,
    server: Option<String>,
    offline: bool,
    cwd_scoped: bool,
) -> Result<()> {
    let full = options.full;
    let mut action = options.action;
    anyhow::ensure!(
        !options.id_only || matches!(action, Action::Start(_) | Action::Create(_)),
        "--id-only applies to delegation start/create"
    );
    if matches!(context::role(None)?.as_str(), "guest")
        && !matches!(
            &action,
            Action::Sync(_)
                | Action::Status(_)
                | Action::Wait(_)
                | Action::Ack { .. }
                | Action::Publish(_)
                | Action::Mcp
                | Action::Guide { guest: true }
                | Action::Worker { .. }
        )
    {
        anyhow::bail!("guest processes cannot perform host actions");
    }
    super::resolve::action(&mut action, full)?;
    let result_only = matches!(
        &action,
        Action::Result(_)
            | Action::Wait(args::WaitArgs {
                until: args::Until::Result,
                ..
            })
    );
    let diagnostics = matches!(&action, Action::Output { .. });
    let value: Value = match action {
        Action::Start(args) => super::create::run(args, project, server, true, offline)?,
        Action::Create(args) => super::create::run(args, project, server, false, offline)?,
        Action::Run(args) => return super::providers::synchronous(args),
        Action::Sync(args) => super::reads::snapshot(&args, false)?,
        Action::Status(args) => super::reads::snapshot(&args, false)?,
        Action::Wait(args) => super::reads::wait(args, false)?,
        Action::Result(args) => super::reads::wait(args, true)?,
        Action::Ack { context, through } => super::reads::ack(context, through)?,
        Action::Publish(args) => super::publish::run(args)?,
        Action::Send(args) => publish(args, "message")?,
        Action::Answer(args) => publish(args, "answer")?,
        Action::Checkpoint(args) => publish(args, "checkpoint")?,
        Action::Recover { id, stall_timeout } => {
            super::worker::launch_with_timeout(&id, stall_timeout)?
        }
        Action::Output { id, lines } => super::diagnostics::output(&id, lines)?,
        Action::Integrate { id } => super::lifecycle::integrate(&id)?,
        Action::Cancel { id, reason } => super::lifecycle::cancel(&id, &reason)?,
        Action::List { active, all, limit } => {
            super::maintenance::list(active || !all, limit, cwd_scoped)?
        }
        Action::Purge { id, force } => super::maintenance::purge(&id, force)?,
        Action::Gc { days } => super::maintenance::gc(days)?,
        Action::Models { target } => super::models::run(target)?,
        Action::Guide { guest } => {
            crate::stdout::println!("{}", super::guide::body(guest));
            return Ok(());
        }
        Action::Mcp => return super::mcp::run(offline),
        Action::McpConfig { id } => {
            context::host_only()?;
            store::get(&store::open()?, &id)?;
            super::providers::adapters::mcp_config(&std::env::current_exe()?)
        }
        Action::Migrate { path, merge } => super::migrate::run(&path, merge)?,
        Action::Worker { id } => return super::worker::run(&id),
    };
    if options.id_only {
        crate::stdout::println!("{}", super::text(&value["session"], "id"));
        return Ok(());
    }
    crate::output::json(&if full || diagnostics {
        value
    } else {
        super::render::compact(value, result_only)
    })
}

fn publish(body: args::BodyArgs, kind: &str) -> Result<Value> {
    super::publish::run(args::PublishArgs {
        body,
        kind: kind.to_owned(),
    })
}
