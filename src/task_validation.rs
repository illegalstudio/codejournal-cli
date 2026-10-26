use crate::{cli::Command, task_args::TaskAction};
use anyhow::{Result, ensure};

pub fn preflight(command: &Command) -> Result<()> {
    if let Command::Task {
        action: TaskAction::Edit(args),
    } = command
    {
        let body = args.body.is_some() || args.body_file.is_some() || args.append_body.is_some();
        ensure!(
            !body || args.base.is_some(),
            "task body edits require --base BODY_VERSION; run cj task show {} --json and use task.body_version. Use --append-body TEXT to add context without replacing the body.",
            args.id
        );
        crate::refs::parse_all(&args.refs)?;
    }
    Ok(())
}
