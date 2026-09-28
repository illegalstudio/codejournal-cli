use crate::api::Api;
use crate::task_args::TaskAddArgs;
use crate::{attribution, input, output, project, project_bootstrap, refs};
use anyhow::Result;
use serde_json::json;

pub fn run(
    api: &Api,
    tenant: &str,
    explicit_project: Option<&str>,
    args: TaskAddArgs,
    json_mode: bool,
) -> Result<()> {
    let here = if explicit_project.is_some() {
        project::slug(explicit_project)?
    } else {
        project_bootstrap::ensure(api, tenant, false)?
    };
    let destination = args.target.as_deref().unwrap_or(&here);
    let body = input::optional_body(args.body, args.body_file)?;
    let response = api.post(
        &format!("/api/v1/tenants/{tenant}/projects/{destination}/tasks"),
        &json!({
            "title": args.title, "body": body, "priority": args.priority,
            "refs": refs::parse_all(&args.refs)?, "not_before": args.not_before,
            "source_project": if args.target.is_some() { Some(here.as_str()) } else { None },
            "source_entry_id": args.from_entry, "plan": args.plan,
            "agent": attribution::agent(args.agent.as_deref()), "host": attribution::host(),
        }),
    )?;
    let task = &response["task"];
    let id = task["id"].as_str().unwrap_or("").replace('-', "");
    let short = &id[..id.len().min(8)];
    let text = format!(
        "Created task {short} ({}) in {destination}: {}",
        task["priority"].as_str().unwrap_or("normal"),
        task["title"].as_str().unwrap_or("")
    );
    output::emit(&json!({"task": task, "queued": false}), &text, json_mode)
}
