use crate::api::Api;
use crate::search_args::{RecentArgs, SearchArgs};
use crate::{project_bootstrap, staleness};
use anyhow::Result;

mod empty;
mod format;
mod response;

fn root(tenant: &str) -> String {
    format!("/api/v1/tenants/{tenant}/entries")
}

pub fn run(
    api: &Api,
    tenant: &str,
    explicit_project: Option<&str>,
    args: SearchArgs,
    json_mode: bool,
) -> Result<()> {
    let mut params = vec![
        ("limit", args.limit.to_string()),
        ("summary", if args.verbose { "0" } else { "1" }.to_owned()),
    ];
    if !args.all_projects {
        params.push((
            "project",
            project_bootstrap::resolved_slug(api, tenant, explicit_project)?,
        ));
    }
    if args.query.is_some() {
        params.push(("excerpt", "1".to_owned()));
    }
    if args.literal {
        params.push(("literal", "1".to_owned()));
    }
    if args.whole_word {
        params.push(("whole_word", "1".to_owned()));
    }
    for (key, value) in [
        ("q", args.query),
        ("kind", args.kind),
        ("topic", args.topic),
        ("path", args.path),
    ] {
        if let Some(value) = value {
            params.push((key, value));
        }
    }
    if args.any {
        params.push(("any", "1".to_owned()));
    }
    if args.prefix {
        params.push(("prefix", "1".to_owned()));
    }
    if args.all_statuses {
        params.push(("status", "all".to_owned()));
    }
    fetch(api, tenant, params, json_mode, args.all_projects)
}

pub fn recent(
    api: &Api,
    tenant: &str,
    explicit_project: Option<&str>,
    args: RecentArgs,
    json_mode: bool,
) -> Result<()> {
    let slug = project_bootstrap::resolved_slug(api, tenant, explicit_project)?;
    let mut params = vec![
        ("project", slug),
        ("limit", args.limit.to_string()),
        ("summary", if args.verbose { "0" } else { "1" }.to_owned()),
    ];
    if let Some(kind) = args.kind {
        params.push(("kind", kind));
    }
    if args.all {
        params.push(("status", "all".to_owned()));
    }
    fetch(api, tenant, params, json_mode, false)
}

fn fetch(
    api: &Api,
    tenant: &str,
    params: Vec<(&str, String)>,
    json_mode: bool,
    all_projects: bool,
) -> Result<()> {
    let query = reqwest::Url::parse_with_params("http://local/", &params)?;
    let mut response = api.get(&format!("{}?{}", root(tenant), query.query().unwrap_or("")))?;
    anyhow::ensure!(
        !params
            .iter()
            .any(|(key, value)| *key == "whole_word" && value == "1")
            || response["search_mode"] == "whole_word",
        "this server does not support whole-word search; update the server and retry"
    );
    if !all_projects {
        if let Some((_, slug)) = params.iter().find(|(key, _)| *key == "project") {
            staleness::enrich(
                api,
                &format!("/api/v1/tenants/{tenant}/projects/{slug}"),
                &mut response,
            )?;
        }
    }
    crate::output::discovery::list(
        &mut response,
        "entries",
        params
            .iter()
            .find(|(key, _)| *key == "status")
            .map_or("active", |(_, value)| value.as_str()),
        params
            .iter()
            .any(|(key, value)| *key == "summary" && value == "0"),
    );
    if params
        .iter()
        .any(|(key, value)| *key == "summary" && value == "1")
        && let Some(fields) = response["project"].as_object_mut()
    {
        fields.retain(|key, _| matches!(key.as_str(), "id" | "slug" | "name"));
    }
    response::emit(response, &params, all_projects, json_mode)
}
