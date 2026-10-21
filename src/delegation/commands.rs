use super::{
    args::{self, Action},
    context, store,
};
use anyhow::Result;
use serde_json::Value;

pub fn run(
    action: Action,
    project: Option<String>,
    server: Option<String>,
    offline: bool,
) -> Result<()> {
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
    let value: Value = match action {
        Action::Start(args) => super::create::run(args, project, server, true, offline)?,
        Action::Create(args) => super::create::run(args, project, server, false, offline)?,
        Action::Run(args) => return super::providers::synchronous(args),
        Action::Sync(args) => super::reads::snapshot(&args, false)?,
        Action::Status(mut args) => {
            if args.after.is_none() {
                args.full = true;
            }
            super::reads::snapshot(&args, false)?
        }
        Action::Wait(args) => super::reads::wait(args, false)?,
        Action::Result(args) => super::reads::wait(args, true)?,
        Action::Ack { context, through } => super::reads::ack(context, through)?,
        Action::Publish(args) => super::publish::run(args)?,
        Action::Send(args) => publish(args, "message")?,
        Action::Answer(args) => publish(args, "answer")?,
        Action::Checkpoint(args) => publish(args, "checkpoint")?,
        Action::Recover { id } => super::worker::launch(&id, true)?,
        Action::Integrate { id } => super::lifecycle::integrate(&id)?,
        Action::Cancel { id, reason } => super::lifecycle::cancel(&id, &reason)?,
        Action::List { active, all, limit } => super::maintenance::list(active || !all, limit)?,
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
    crate::output::json(&value)
}

fn publish(body: args::BodyArgs, kind: &str) -> Result<Value> {
    super::publish::run(args::PublishArgs {
        body,
        kind: kind.to_owned(),
    })
}
