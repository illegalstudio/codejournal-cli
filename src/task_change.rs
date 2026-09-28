use crate::api::Api;
use crate::task_args::TaskEditArgs;
use crate::{attribution, input, output, project, refs};
use anyhow::{Result, bail};
use serde_json::{Value, json};

fn path(tenant: &str, explicit_project: Option<&str>, id: &str) -> Result<String> {
    Ok(format!(
        "/api/v1/tenants/{tenant}/projects/{}/tasks/{id}",
        project::slug(explicit_project)?
    ))
}

fn mutate(
    api: &Api,
    tenant: &str,
    project: Option<&str>,
    id: &str,
    data: Value,
    text: String,
    json_mode: bool,
) -> Result<()> {
    let result = api.patch(&path(tenant, project, id)?, &data)?;
    output::emit(&result, &text, json_mode)
}

pub fn status(
    api: &Api,
    tenant: &str,
    project: Option<&str>,
    id: &str,
    action: &str,
    note: Option<String>,
    agent: Option<String>,
    json_mode: bool,
) -> Result<()> {
    if ["done", "dismiss"].contains(&action) && note.as_deref().is_none_or(str::is_empty) {
        bail!("say what changed (done) or why not (dismiss) with --note");
    }
    let status = match action {
        "start" => "in_progress",
        "reopen" => "open",
        other => other,
    };
    mutate(
        api,
        tenant,
        project,
        id,
        json!({"action": action, "note": note,
        "agent": attribution::agent(agent.as_deref()), "host": attribution::host()}),
        format!("Task {} is now {status}.", short(id)),
        json_mode,
    )
}

pub fn note(
    api: &Api,
    tenant: &str,
    project: Option<&str>,
    id: &str,
    note: Option<String>,
    body_file: Option<String>,
    agent: Option<String>,
    json_mode: bool,
) -> Result<()> {
    let note = input::optional_body(note, body_file)?;
    if note.is_empty() {
        bail!("a note needs text: --note TEXT or stdin");
    }
    mutate(
        api,
        tenant,
        project,
        id,
        json!({"action": "note", "note": note,
        "agent": attribution::agent(agent.as_deref()), "host": attribution::host()}),
        format!("Added a note to task {}.", short(id)),
        json_mode,
    )
}

pub fn refs(
    api: &Api,
    tenant: &str,
    project: Option<&str>,
    id: &str,
    raw_refs: Vec<String>,
    agent: Option<String>,
    unlink: bool,
    json_mode: bool,
) -> Result<()> {
    let parsed = refs::parse_all(&raw_refs)?;
    let verb = if unlink { "unlinked" } else { "linked" };
    mutate(
        api,
        tenant,
        project,
        id,
        json!({"action": if unlink { "unlink" } else { "link" },
        "refs": parsed, "agent": attribution::agent(agent.as_deref()), "host": attribution::host()}),
        format!("Task {}: {verb} {}.", short(id), raw_refs.join(", ")),
        json_mode,
    )
}

pub fn edit(
    api: &Api,
    tenant: &str,
    project: Option<&str>,
    args: TaskEditArgs,
    json_mode: bool,
) -> Result<()> {
    if args.title.is_none()
        && args.priority.is_none()
        && args.not_before.is_none()
        && args.plan.is_none()
        && !args.no_plan
    {
        bail!("nothing to change; pass --title, --priority, --plan, --no-plan, or --not-before");
    }
    let id = args.id;
    let mut data = json!({"action": "edit", "no_plan": args.no_plan,
        "agent": attribution::agent(args.agent.as_deref()), "host": attribution::host()});
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
    mutate(
        api,
        tenant,
        project,
        &id,
        data,
        format!("Updated task {}.", short(&id)),
        json_mode,
    )
}

fn short(id: &str) -> String {
    id.replace('-', "").chars().take(8).collect()
}
