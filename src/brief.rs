use crate::api::Api;
use crate::{
    attribution, brief_audit, brief_focus, brief_format, output, project, session_state, staleness,
};
use anyhow::Result;
use clap::Args;
use serde_json::{Value, json};

#[derive(Args)]
pub struct BriefArgs {
    /// Recent entries, 0-200; use 0 to omit this section.
    #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u32).range(0..=200))]
    pub limit: u32,
    /// Pinned entries, 0-200; use 0 to omit this section.
    #[arg(long, default_value_t = 15, value_parser = clap::value_parser!(u32).range(0..=200))]
    pub pinned_limit: u32,
    /// Recent logs, 0-200; use 0 to omit this section.
    #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u32).range(0..=200))]
    pub log_limit: u32,
    /// Shorten text output further; JSON uses summaries by default.
    #[arg(long)]
    pub compact: bool,
    /// Include inactive records explicitly.
    #[arg(long)]
    pub all: bool,
    /// Include full content and metadata in JSON.
    #[arg(short, long)]
    pub verbose: bool,
    /// Read only complete rules, active sessions and project task/plan/doc metadata, without global content or maintenance suggestions.
    #[arg(long, conflicts_with_all = ["compact", "max_chars"])]
    pub audit: bool,
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
        (
            "visibility",
            if args.all { "all" } else { "active" }.to_owned(),
        ),
        ("summary", if args.verbose { "0" } else { "1" }.to_owned()),
    ];
    if let Some(session) = &session {
        params.push(("session", session.clone()));
    }
    if args.audit {
        params.push(("audit", "1".to_owned()));
    }
    let query = reqwest::Url::parse_with_params("http://local/", &params)?;
    let mut result = brief_focus::load(
        api,
        &path,
        &format!("{path}/brief?{}", query.query().unwrap_or("")),
        json!({"limit": args.limit, "pinned_limit": args.pinned_limit,
            "log_limit": args.log_limit, "agent": agent, "session": session, "audit": args.audit,
            "visibility": if args.all { "all" } else { "active" }, "summary": !args.verbose}),
    )?;
    if args.audit {
        let audit = brief_audit::project(&result);
        return output::emit(&audit, &brief_audit::render(&audit), json);
    }
    if name.is_none() {
        result["entries"] = result["recent"].clone();
        staleness::enrich(api, &path, &mut result)?;
        result["recent"] = result["entries"].clone();
    }
    if !args.verbose
        && let Some(fields) = result.as_object_mut()
    {
        fields.remove("entries");
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
