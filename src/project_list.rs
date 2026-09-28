use crate::api::Api;
use crate::output;
use anyhow::Result;
use serde_json::{Value, json};

pub fn run(api: &Api, tenant: &str, json_mode: bool) -> Result<()> {
    let response = api.get(&format!("/api/v1/tenants/{tenant}/projects"))?;
    let projects = response["projects"].as_array().cloned().unwrap_or_default();
    let lines: Vec<_> = projects.iter().map(line).collect();
    let text = if lines.is_empty() {
        "  (no projects yet)".to_owned()
    } else {
        lines.join("\n")
    };
    let payload = json!({"projects": projects.iter().map(|project| json!({
        "slug": project["slug"], "name": project["name"],
        "remote_url": project["remote_url"], "active_entries": project["active_entries"],
        "has_rules": project["has_rules"],
    })).collect::<Vec<_>>()});
    output::emit(&payload, &text, json_mode)
}

fn line(project: &Value) -> String {
    let remote = project["remote_url"].as_str().unwrap_or("(no remote)");
    format!(
        "  {:<32} {:>4} active  {:<8}  {remote}",
        project["slug"].as_str().unwrap_or(""),
        project["active_entries"].as_u64().unwrap_or(0),
        if project["has_rules"].as_bool().unwrap_or(false) {
            "rules"
        } else {
            "no rules"
        }
    )
}
