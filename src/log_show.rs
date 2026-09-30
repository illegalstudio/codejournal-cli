use crate::{api::Api, output};
use anyhow::Result;
use serde_json::Value;

pub fn run(api: &Api, tenant: &str, id: &str, body_only: bool, json_mode: bool) -> Result<()> {
    let result = api.get(&format!("/api/v1/tenants/{tenant}/logs/{id}"))?;
    let log = &result["log"];
    if body_only && !json_mode {
        println!("{}", text(&log["body"]));
        return Ok(());
    }
    let mut lines = vec![
        format!("id:        {}", text(&log["id"]).replace('-', "")),
        format!("project:   {}", text(&log["project_slug"])),
        format!("status:    {}", text(&log["status"])),
        format!("created:   {}", text(&log["created_at"])),
        format!("agent:     {}", text(&log["agent"])),
    ];
    if let Some(plan) = log["plan_id"].as_str() {
        lines.push(format!("plan:      {}", plan.replace('-', "")));
    }
    for reference in log["refs"].as_array().into_iter().flatten() {
        lines.push(format!(
            "ref:       {}:{}",
            text(&reference["kind"]),
            text(&reference["value"])
        ));
    }
    lines.extend([
        String::new(),
        text(&log["title"]).to_owned(),
        String::new(),
        text(&log["body"]).to_owned(),
    ]);
    output::emit(&result, &lines.join("\n"), json_mode)
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
