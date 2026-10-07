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
    let doc = matches!(command, Command::Doc { .. });
    let (Command::Plan { action } | Command::Doc { action }) = command else {
        return Ok(());
    };
    if let PlanAction::List {
        status: Some(status),
        ..
    } = action
    {
        let accepted: &[&str] = if doc {
            &["current", "draft", "outdated", "open", "all"]
        } else {
            &["active", "draft", "done", "abandoned", "open", "all"]
        };
        if !accepted.contains(&status.as_str()) {
            bail!(
                "invalid {} list status {status}; use {}",
                if doc { "doc" } else { "plan" },
                accepted.join(", ")
            );
        }
    }
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
