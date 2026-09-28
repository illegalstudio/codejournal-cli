use crate::api::Api;
use crate::output;
use anyhow::Result;
use serde_json::{Value, json};

pub fn show(api: &Api, tenant: &str, id: &str, json_mode: bool) -> Result<()> {
    let response = api.get(&format!("/api/v1/tenants/{tenant}/tasks/{id}"))?;
    let task = &response["task"];
    let mut lines = vec![
        format!("id:        {}", string(&task["id"]).replace('-', "")),
        format!("project:   {}", string(&task["project_slug"])),
        format!(
            "status:    {}{}",
            string(&task["status"]),
            task["resolution"]
                .as_str()
                .map(|note| format!(" ({note})"))
                .unwrap_or_default()
        ),
        format!("priority:  {}", string(&task["priority"])),
        format!(
            "created:   {} by {}",
            string(&task["created_at"]),
            task["agent"].as_str().unwrap_or("unknown")
        ),
    ];
    if task["forwarded"].as_bool().unwrap_or(false) {
        lines.push(format!("from:      {}", string(&task["source_slug"])));
    }
    if let Some(plan) = task["plan_id"].as_str() {
        lines.push(format!(
            "plan:      {} {}",
            short(plan),
            string(&task["plan_title"])
        ));
    }
    if let Some(refs) = task["refs"].as_array() {
        for (index, item) in refs.iter().enumerate() {
            lines.push(format!(
                "{}{}:{}",
                if index == 0 {
                    "links:     "
                } else {
                    "           "
                },
                string(&item["kind"]),
                string(&item["value"])
            ));
        }
    }
    lines.extend([
        "".to_owned(),
        string(&task["title"]).to_owned(),
        "".to_owned(),
        task["body"]
            .as_str()
            .filter(|body| !body.is_empty())
            .unwrap_or("(no details)")
            .to_owned(),
        "".to_owned(),
        "History:".to_owned(),
    ]);
    if let Some(events) = task["events"].as_array() {
        for event in events {
            lines.push(format!(
                "  {}  {:<8} {}{}",
                &string(&event["created_at"])[..16.min(string(&event["created_at"]).len())],
                event["agent"].as_str().unwrap_or("unknown"),
                string(&event["kind"]),
                event["note"]
                    .as_str()
                    .map(|note| format!(": {note}"))
                    .unwrap_or_default()
            ));
        }
    }
    output::emit(
        &json!({"task": task, "events": task["events"]}),
        &lines.join("\n"),
        json_mode,
    )
}

fn short(id: &str) -> String {
    id.replace('-', "").chars().take(8).collect()
}
fn string(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
