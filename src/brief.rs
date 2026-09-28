use crate::api::Api;
use crate::{attribution, brief_focus, brief_format, output, project, session_state, staleness};
use anyhow::Result;
use clap::Args;
use serde_json::{Value, json};

#[derive(Args)]
pub struct BriefArgs {
    #[arg(long, default_value_t = 10)]
    pub limit: u32,
    #[arg(long, default_value_t = 15)]
    pub pinned_limit: u32,
    #[arg(long, default_value_t = 5)]
    pub log_limit: u32,
    #[arg(long)]
    pub compact: bool,
    #[arg(long)]
    pub max_chars: Option<usize>,
    #[arg(long)]
    pub agent: Option<String>,
    #[arg(long, hide = true)]
    pub session_key: Option<String>,
}

pub fn run(api: &Api, tenant: &str, name: Option<&str>, args: BriefArgs, json: bool) -> Result<()> {
    let path = format!("/api/v1/tenants/{tenant}/projects/{}", project::slug(name)?);
    let agent = attribution::agent(args.agent.as_deref());
    let session = args.session_key.or_else(session_state::current_id);
    let mut params = vec![
        ("limit", args.limit.to_string()),
        ("pinned_limit", args.pinned_limit.to_string()),
        ("log_limit", args.log_limit.to_string()),
        ("agent", agent.clone()),
    ];
    if let Some(session) = &session {
        params.push(("session", session.clone()));
    }
    let query = reqwest::Url::parse_with_params("http://local/", &params)?;
    let mut result = brief_focus::load(
        api,
        &path,
        &format!("{path}/brief?{}", query.query().unwrap_or("")),
        json!({"limit": args.limit, "pinned_limit": args.pinned_limit,
            "log_limit": args.log_limit, "agent": agent, "session": session}),
    )?;
    if name.is_none() {
        staleness::enrich(api, &path, &mut result)?;
    }
    let full = brief_format::render(
        &result,
        false,
        args.limit,
        args.pinned_limit,
        args.log_limit,
    );
    let compact = args.compact || args.max_chars.is_some_and(|max| full.len() > max);
    result["compact"] = Value::Bool(compact);
    output::emit(
        &result,
        &if compact {
            brief_format::render(&result, true, args.limit, args.pinned_limit, args.log_limit)
        } else {
            full
        },
        json,
    )
}
