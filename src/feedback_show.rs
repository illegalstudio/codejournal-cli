use crate::{api::Api, output};
use anyhow::Result;
use serde_json::Value;

fn string(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}

pub fn show(api: &Api, tenant: &str, id: &str, json_mode: bool) -> Result<()> {
    let response = api.get(&format!("/api/v1/tenants/{tenant}/feedback/{id}"))?;
    let item = &response["feedback"];
    let mut lines = vec![
        format!("id:        {}", string(&item["id"]).replace('-', "")),
        format!(
            "project:   {}",
            item["project_slug"].as_str().unwrap_or("no project")
        ),
        format!("status:    {}", string(&item["status"])),
        format!("category:  {}", string(&item["category"])),
        format!("agent:     {}", item["agent"].as_str().unwrap_or("unknown")),
        format!("host:      {}", item["host"].as_str().unwrap_or("unknown")),
        format!("created:   {}", string(&item["created_at"])),
        format!("updated:   {}", string(&item["updated_at"])),
        String::new(),
        string(&item["title"]).to_owned(),
        String::new(),
        item["body"].as_str().unwrap_or("(no details)").to_owned(),
    ];
    if let Some(resolution) = item["resolution"].as_str().filter(|note| !note.is_empty()) {
        lines.extend([String::new(), format!("resolution: {resolution}")]);
    }
    output::emit(&response, &lines.join("\n"), json_mode)
}
