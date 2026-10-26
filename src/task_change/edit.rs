use crate::{api::Api, attribution, input, task_args::TaskEditArgs};
use anyhow::{Result, bail};
use serde_json::{Value, json};

pub fn run(
    api: &Api,
    tenant: &str,
    project: Option<&str>,
    args: TaskEditArgs,
    json_mode: bool,
) -> Result<()> {
    let has_body = args.body.is_some() || args.body_file.is_some();
    if args.title.is_none()
        && args.priority.is_none()
        && args.not_before.is_none()
        && args.plan.is_none()
        && !args.no_plan
        && !has_body
        && args.append_body.is_none()
        && args.refs.is_empty()
        && !args.clear_refs
    {
        bail!(
            "nothing to change; pass --title, --priority, --plan, --no-plan, --not-before, --ref/--clear-refs or a body edit"
        );
    }
    let mut data = json!({"action": "edit", "no_plan": args.no_plan,
        "agent": attribution::agent(args.agent.as_deref()), "host": attribution::host()});
    if has_body {
        let body = input::optional_body(args.body, args.body_file)?;
        anyhow::ensure!(
            body.chars().count() <= 100_000,
            "task body exceeds 100000 characters"
        );
        data["body"] = json!(body);
        data["based_on"] = json!(args.base);
    }
    if let Some(body) = args.append_body {
        anyhow::ensure!(
            !body.trim().is_empty() && body.chars().count() <= 100_000,
            "appended task context must contain 1 to 100000 characters"
        );
        data["append_body"] = json!(body);
        data["based_on"] = json!(args.base);
    }
    if !args.refs.is_empty() || args.clear_refs {
        data["refs"] = json!(crate::refs::parse_all(&args.refs)?);
    }
    for (key, value) in [
        ("title", args.title),
        ("priority", args.priority),
        ("not_before", args.not_before),
        ("plan", args.plan),
    ] {
        if let Some(value) = value {
            data[key] = Value::String(value);
        }
    }
    super::mutate(
        api,
        tenant,
        project,
        &args.id,
        data,
        format!("Updated task {}.", super::short(&args.id)),
        json_mode,
    )
}
