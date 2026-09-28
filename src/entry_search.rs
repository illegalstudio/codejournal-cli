use crate::api::Api;
use crate::search_args::SearchArgs;
use crate::{output, project, staleness};
use anyhow::Result;
use serde_json::{Value, json};

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
    let mut params = vec![("limit", args.limit.to_string())];
    if !args.all_projects {
        params.push(("project", project::slug(explicit_project)?));
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
    limit: u32,
    kind: Option<&str>,
    json_mode: bool,
) -> Result<()> {
    let slug = project::slug(explicit_project)?;
    let mut params = vec![("project", slug), ("limit", limit.to_string())];
    if let Some(kind) = kind {
        params.push(("kind", kind.to_owned()));
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
    let entries = response["entries"].as_array().cloned().unwrap_or_default();
    let lines: Vec<_> = entries
        .iter()
        .map(|entry| entry_line(entry, all_projects))
        .collect();
    let payload = json!({
        "project": response["project"], "entries": entries, "notices": [],
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

fn entry_line(entry: &Value, all_projects: bool) -> String {
    let id = string(&entry["id"]).replace('-', "");
    let id = &id[..id.len().min(8)];
    let date = string(&entry["created_at"]);
    let date = &date[..date.len().min(10)];
    let topics = array_text(&entry["topics"]);
    let topic_suffix = if topics.is_empty() {
        String::new()
    } else {
        format!("  [{topics}]")
    };
    let status = string(&entry["status"]);
    let status_suffix = if status == "active" {
        String::new()
    } else {
        format!("  ({status})")
    };
    let global = if string(&entry["scope"]) == "global" {
        "  [global]"
    } else {
        ""
    };
    let wrong = entry["usage"]["wrong"].as_u64().unwrap_or(0);
    let wrong = if wrong > 0 {
        format!("  [reported wrong {wrong}x]")
    } else {
        String::new()
    };
    let prefix = if all_projects {
        format!("  [{}]", string(&entry["project_slug"]))
    } else {
        "  ".to_owned()
    };
    format!(
        "{prefix}{id}  {:<12} {date}  {}{topic_suffix}{status_suffix}{global}{wrong}",
        string(&entry["kind"]),
        string(&entry["title"])
    )
}

fn string(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
fn array_text(value: &Value) -> String {
    value
        .as_array()
        .map(|items| items.iter().map(string).collect::<Vec<_>>().join(", "))
        .unwrap_or_default()
}
