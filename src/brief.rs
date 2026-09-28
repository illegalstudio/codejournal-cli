use crate::api::Api;
use crate::{brief_format, output, project, staleness};
use anyhow::Result;
use clap::Args;
use serde_json::Value;

#[derive(Args)]
pub struct BriefArgs {
    #[arg(long, default_value_t = 12)]
    pub limit: u32,
    #[arg(long, default_value_t = 10)]
    pub pinned_limit: u32,
    #[arg(long, default_value_t = 8)]
    pub log_limit: u32,
    #[arg(long)]
    pub compact: bool,
    #[arg(long)]
    pub max_chars: Option<usize>,
    #[arg(long)]
    pub agent: Option<String>,
}

pub fn run(api: &Api, tenant: &str, name: Option<&str>, args: BriefArgs, json: bool) -> Result<()> {
    let path = format!("/api/v1/tenants/{tenant}/projects/{}", project::slug(name)?);
    let mut result = api.get(&format!(
        "{path}/brief?limit={}&pinned_limit={}&log_limit={}",
        args.limit, args.pinned_limit, args.log_limit
    ))?;
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
