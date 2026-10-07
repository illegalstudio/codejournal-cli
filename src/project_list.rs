use crate::api::Api;
use crate::output;
use anyhow::Result;
use serde_json::{Value, json};

pub fn run(
    api: &Api,
    tenant: &str,
    args: crate::project_args::ProjectListArgs,
    json_mode: bool,
) -> Result<()> {
    let status = if args.all {
        "all"
    } else if args.archived {
        "archived"
    } else {
        "active"
    };
    let query = if status == "active" {
        String::new()
    } else {
        format!("?status={status}")
    };
    let response = api.get(&format!("/api/v1/tenants/{tenant}/projects{query}"))?;
    let projects = response["projects"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|project| args.all || (project["archived_at"].is_string() == args.archived))
        .collect::<Vec<_>>();
    let lines: Vec<_> = projects.iter().map(line).collect();
    let text = if lines.is_empty() {
        "  (no projects yet)".to_owned()
    } else {
        lines.join("\n")
    };
    let payload = json!({"projects": projects.iter().map(|project| json!({
        "slug": project["slug"], "name": project["name"],
        "remote_url": project["remote_url"], "active_entries": project["active_entries"],
        "archived_at": project["archived_at"], "has_rules": project["has_rules"], "locked": project["locked"].as_bool().unwrap_or(false),
    })).collect::<Vec<_>>()});
    output::emit(&payload, &text, json_mode)
}

fn line(project: &Value) -> String {
    let remote = project["remote_url"].as_str().unwrap_or("(no remote)");
    // Free workspaces read one project; the others keep their writes until a switch or upgrade.
    let locked = if project["archived_at"].is_string() {
        "  (archived, read-only)"
    } else if project["locked"] == true {
        "  (locked on Free)"
    } else {
        ""
    };
    format!(
        "  {:<32} {:>4} active  {:<8}  {remote}{locked}",
        project["slug"].as_str().unwrap_or(""),
        project["active_entries"].as_u64().unwrap_or(0),
        if project["has_rules"].as_bool().unwrap_or(false) {
            "rules"
        } else {
            "no rules"
        }
    )
}
