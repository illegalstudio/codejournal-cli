use crate::api::Api;
use crate::task_args::TaskListArgs;
use crate::{output, project};
use anyhow::Result;
use serde_json::Value;

pub fn list(
    api: &Api,
    tenant: &str,
    explicit_project: Option<&str>,
    args: TaskListArgs,
    json_mode: bool,
) -> Result<()> {
    let base = if args.all_projects {
        format!("/api/v1/tenants/{tenant}/tasks")
    } else {
        format!(
            "/api/v1/tenants/{tenant}/projects/{}/tasks",
            project::slug(explicit_project)?
        )
    };
    let mut params = vec![("status", args.status)];
    if let Some(source) = args.source {
        params.push(("from", source));
    }
    if let Some(priority) = args.priority {
        params.push(("priority", priority));
    }
    if args.forwarded {
        params.push(("forwarded", "1".to_owned()));
    }
    let query = reqwest::Url::parse_with_params("http://local/", &params)?;
    let result = api.get(&format!("{base}?{}", query.query().unwrap_or("")))?;
    let lines: Vec<_> = result["tasks"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|task| line(task, args.all_projects))
        .collect();
    output::emit(
        &result,
        &if lines.is_empty() {
            "  (no tasks)".to_owned()
        } else {
            lines.join("\n")
        },
        json_mode,
    )
}

fn line(task: &Value, all_projects: bool) -> String {
    let project = if all_projects {
        format!("{:<24} ", string(&task["project_slug"]))
    } else {
        String::new()
    };
    let priority = if string(&task["priority"]) == "normal" {
        String::new()
    } else {
        format!("[{}] ", string(&task["priority"]))
    };
    let origin = if task["forwarded"].as_bool().unwrap_or(false) {
        format!("  (from {})", string(&task["source_slug"]))
    } else {
        String::new()
    };
    let plan = task["plan_id"]
        .as_str()
        .map(|id| format!("  (plan {})", short(id)))
        .unwrap_or_default();
    format!(
        "  {}  {:<11} {project}{priority}{}{origin}{plan}",
        short(string(&task["id"])),
        string(&task["status"]),
        string(&task["title"])
    )
}

fn short(id: &str) -> String {
    id.replace('-', "").chars().take(8).collect()
}
fn string(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
