use crate::{cli::Command, input, plan_args::PlanAction, refs};
use anyhow::{Result, bail};

pub fn body(markdown: &str) -> Result<()> {
    if markdown.chars().count() > 100_000 {
        bail!(
            "document/plan body exceeds 100000 characters; split it into focused documents or summarize it. Nothing was sent or queued"
        );
    }
    Ok(())
}

pub fn preflight(command: &mut Command) -> Result<()> {
    let (Command::Plan { action } | Command::Doc { action }) = command else {
        return Ok(());
    };
    if let PlanAction::Create {
        body,
        body_file,
        refs: raw,
        ..
    }
    | PlanAction::Update {
        body,
        body_file,
        refs: raw,
        ..
    } = action
    {
        refs::parse_all(raw)?;
        let markdown = input::optional_body(body.take(), body_file.take())?;
        self::body(&markdown)?;
        *body = Some(markdown);
    }
    Ok(())
}
