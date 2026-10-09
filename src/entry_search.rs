use crate::api::Api;
use crate::search_args::{RecentArgs, SearchArgs};
use crate::{output, project_bootstrap, staleness};
use anyhow::Result;
use serde_json::json;

mod format;

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
    let entries = response["entries"].as_array().cloned().unwrap_or_default();
    let lines: Vec<_> = entries
        .iter()
        .map(|entry| {
            let mut line = format::line(entry, all_projects);
            if params
                .iter()
                .any(|(key, value)| *key == "summary" && value == "0")
            {
                for detail in entry["body"].as_str().unwrap_or("").lines() {
                    line.push_str(&format!("\n      {detail}"));
                }
            }
            line
        })
        .collect();
    let payload = json!({
        "project": response["project"], "entries": entries, "cached": response["cached"] == true, "notices": [],
    });
    output::emit(
        &payload,
        &if lines.is_empty() {
            "No matching entries.".to_owned()
        } else {
            lines.join("\n")
        },
        json_mode,
    )
}
