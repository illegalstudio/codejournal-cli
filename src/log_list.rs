use crate::api::Api;
use crate::log_args::LogListArgs;
use crate::{output, project};
use anyhow::Result;
use serde_json::Value;

pub fn run(
    api: &Api,
    tenant: &str,
    explicit_project: Option<&str>,
    args: LogListArgs,
    json_mode: bool,
) -> Result<()> {
    let base = if args.all_projects {
        format!("/api/v1/tenants/{tenant}/logs")
    } else {
        format!(
            "/api/v1/tenants/{tenant}/projects/{}/logs",
            project::slug(explicit_project)?
        )
    };
    let mut params = vec![("limit", args.limit.to_string())];
    for (key, value) in [
        ("since", args.since),
        ("agent", args.agent),
        ("status", args.status),
        ("plan", args.plan),
        ("grep", args.grep),
    ] {
        if let Some(value) = value {
            params.push((key, value));
        }
    }
    let query = reqwest::Url::parse_with_params("http://local/", &params)?;
    let result = api.get(&format!("{base}?{}", query.query().unwrap_or("")))?;
    let mut lines = Vec::new();
    for log in result["logs"].as_array().into_iter().flatten() {
        lines.push(line(log, args.all_projects));
        if args.verbose {
            for detail in text(&log["body"]).lines() {
                lines.push(format!("      {detail}"));
            }
        }
    }
    output::emit(
        &result,
        &if lines.is_empty() {
            "  (no work logged in this window)".to_owned()
        } else {
            lines.join("\n")
        },
        json_mode,
    )
}

fn line(log: &Value, all_projects: bool) -> String {
    let id = text(&log["id"]).replace('-', "");
    let date = text(&log["created_at"]);
    let date = date[..date.len().min(16)].replace('T', " ");
    let project = if all_projects {
        format!("{:<24} ", text(&log["project_slug"]))
    } else {
        String::new()
    };
    let status = if text(&log["status"]) == "done" {
        String::new()
    } else {
        format!(" [{}]", text(&log["status"]))
    };
    let plan = log["plan_id"]
        .as_str()
        .map(|id| format!("  (plan {})", &id.replace('-', "")[..8]))
        .unwrap_or_default();
    format!(
        "  {}  {date}  {project}{:<8} {}{status}{plan}",
        &id[..id.len().min(8)],
        log["agent"].as_str().unwrap_or("unknown"),
        text(&log["title"])
    )
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
