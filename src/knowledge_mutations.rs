use crate::api::Api;
use crate::knowledge_args::EntryAction;
use crate::output;
use crate::{attribution, refs};
use anyhow::{Result, bail};
use serde_json::{Value, json};

mod entry_move;

fn path(tenant: &str, id: &str) -> String {
    format!("/api/v1/tenants/{tenant}/entries/{id}")
}

pub fn answer(
    api: &Api,
    tenant: &str,
    id: &str,
    title: &str,
    kind: &str,
    body: &str,
    raw_refs: Vec<String>,
    agent: Option<String>,
    json_mode: bool,
) -> Result<()> {
    if title.trim().is_empty() || body.trim().is_empty() {
        bail!("an answer needs a --title stating the fact and a body");
    }
    let result = api.post(
        &format!("{}/answer", path(tenant, id)),
        &json!({
            "title": title, "kind": kind, "body": body,
            "refs": refs::parse_all(&raw_refs)?,
            "agent": attribution::agent(agent.as_deref()), "host": attribution::host(),
        }),
    )?;
    let question = result["question"].as_str().unwrap_or(id);
    let answer = result["answer"]["id"].as_str().unwrap_or("");
    output::emit(
        &result,
        &format!(
            "Answered {} with {} ({kind}): {title}",
            short(question),
            short(answer)
        ),
        json_mode,
    )
}

fn short(id: &str) -> String {
    id.replace('-', "").chars().take(8).collect()
}

fn patch(
    api: &Api,
    tenant: &str,
    id: &str,
    value: &Value,
    human: String,
    json_mode: bool,
) -> Result<()> {
    let result = api.patch(&path(tenant, id), value)?;
    output::emit(&result, &human, json_mode)
}

pub fn supersede(api: &Api, tenant: &str, id: &str, by: &str, json_mode: bool) -> Result<()> {
    let result = api.patch(&path(tenant, id), &json!({"action": "supersede", "by": by}))?;
    let old = result["entry"]["id"].as_str().unwrap_or(id);
    let new = result["entry"]["superseded_by"].as_str().unwrap_or(by);
    output::emit(
        &result,
        &format!("Marked {} as superseded by {}.", short(old), short(new)),
        json_mode,
    )
}

pub fn obsolete(api: &Api, tenant: &str, id: &str, json_mode: bool) -> Result<()> {
    patch(
        api,
        tenant,
        id,
        &json!({"action": "obsolete"}),
        format!("Marked {} as obsolete.", short(id)),
        json_mode,
    )
}

pub fn entry(api: &Api, tenant: &str, action: EntryAction, json_mode: bool) -> Result<()> {
    match action {
        EntryAction::Move {
            id,
            to,
            path_prefix,
            replace_prefix,
            dry_run,
        } => entry_move::run(
            api,
            tenant,
            &id,
            &to,
            path_prefix,
            replace_prefix,
            dry_run,
            json_mode,
        ),
        EntryAction::Flag {
            id,
            wrong,
            note,
            agent,
            ..
        } => {
            if wrong && note.as_deref().is_none_or(str::is_empty) {
                bail!(
                    "say what is wrong with --note, then supersede or obsolete the entry if you can"
                );
            }
            let kind = if wrong { "wrong" } else { "helpful" };
            let result = api.post(
                &format!("{}/flags", path(tenant, &id)),
                &json!({"kind": kind, "note": note, "agent": agent}),
            )?;
            let mut human = format!("Flagged {} as {kind}.", short(&id));
            if wrong {
                human.push_str(" Supersede it with the correct fact (`cj add`, then `cj supersede`) or mark it obsolete.");
            }
            output::emit(&result, &human, json_mode)
        }
        EntryAction::Scope { id, scope } => {
            if scope != "global" && scope != "project" {
                bail!("scope must be global or project");
            }
            patch(
                api,
                tenant,
                &id,
                &json!({"action": "scope", "scope": scope}),
                format!("Entry {} is now {scope} knowledge.", short(&id)),
                json_mode,
            )
        }
        EntryAction::SetAgent { id, agent } => {
            let result = api.patch(
                &path(tenant, &id),
                &json!({"action": "set-agent", "agent": agent}),
            )?;
            let old = result["previous_agent"].as_str().unwrap_or("unknown");
            output::emit(
                &result,
                &format!("Set the agent of {} to {agent} (was {old}).", short(&id)),
                json_mode,
            )
        }
    }
}
