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
            "task body edits, including --append-body, require --base BODY_VERSION. Run cj task show {} --json, then cj task edit {} --base BODY_VERSION {} TEXT using the returned task.body_version; nothing was sent or queued.",
            args.id,
            args.id,
            if args.append_body.is_some() {
                "--append-body"
            } else {
                "--body"
            }
        );
        crate::refs::parse_all(&args.refs)?;
    }
    Ok(())
}
