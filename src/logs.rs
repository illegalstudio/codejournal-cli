use crate::api::Api;
use crate::log_args::{LogAction, LogAddArgs};
use crate::{attribution, commands, input, log_list, output, refs, session_git, session_state};
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
    let session = session_state::current_id();
    let mut linked = Vec::new();
    if !args.no_auto_commits && !args.refs.iter().any(|item| item.starts_with("commit:")) {
        if let (Some(id), Some(common), Ok(cwd)) = (
            session.as_deref(),
            session_git::current_common_dir(),
            std::env::current_dir(),
        ) {
            let state = session_state::load(id)?;
            if state.repo_common.as_deref() == Some(common.as_str()) {
                linked = state
                    .commits
                    .iter()
                    .filter(|sha| !state.logged.contains(*sha) && session_git::reachable(&cwd, sha))
                    .cloned()
                    .collect();
            }
        }
    }
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
    if let Some(id) = session.filter(|_| !linked.is_empty()) {
        let mut state = session_state::load(&id)?;
        state.logged.extend(linked.clone());
        session_state::save(&id, &state)?;
    }
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
