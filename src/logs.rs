use crate::api::Api;
use crate::log_args::{LogAction, LogAddArgs};
use crate::{attribution, commands, input, log_commits, log_list, log_show, output, refs};
use anyhow::Result;
use serde_json::json;

pub fn run(
    api: &Api,
    tenant: &str,
    project: Option<&str>,
    action: LogAction,
    json_mode: bool,
) -> Result<()> {
    match action {
        LogAction::Add(args) => add(api, tenant, project, args, json_mode),
        LogAction::List(args) => log_list::run(api, tenant, project, args, json_mode),
        LogAction::Show { id, body } => log_show::run(api, tenant, &id, body, json_mode),
    }
}

fn add(
    api: &Api,
    tenant: &str,
    project: Option<&str>,
    args: LogAddArgs,
    json_mode: bool,
) -> Result<()> {
    let body = input::body(args.body, args.body_file)?;
    let linked = log_commits::pending(&args.refs, args.no_auto_commits)?;
    let mut all_refs = args.refs;
    all_refs.extend(linked.iter().map(|sha| format!("commit:{sha}")));
    let result = api.post(
        &format!("{}/logs", commands::path(tenant, project)?),
        &json!({
            "title": args.title, "body": body, "status": args.status, "plan": args.plan,
            "agent": attribution::agent(args.agent.as_deref()), "host": attribution::host(),
            "refs": refs::parse_all(&all_refs)?,
        }),
    )?;
    log_commits::record(&all_refs)?;
    let log = &result["log"];
    let id = log["id"].as_str().unwrap_or("").replace('-', "");
    let mut text = format!(
        "Logged {} ({}): {}",
        &id[..id.len().min(8)],
        log["status"].as_str().unwrap_or("done"),
        log["title"].as_str().unwrap_or("")
    );
    if !linked.is_empty() {
        text.push_str(&format!(
            "\nLinked {} commit(s) made since your previous log: {}",
            linked.len(),
            linked.join(", ")
        ));
    }
    output::emit(&json!({"log": log, "queued": false}), &text, json_mode)
}
