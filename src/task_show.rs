use crate::api::Api;
use crate::output;
use anyhow::Result;
use serde_json::{Value, json};

/// Events shown without --history: enough for the current state, not the whole trail.
const RECENT_EVENTS: usize = 5;

pub fn show(api: &Api, tenant: &str, id: &str, history: bool, json_mode: bool) -> Result<()> {
    let mut response = api.get(&format!("/api/v1/tenants/{tenant}/tasks/{id}"))?;
    let all = match response["task"]["events"].take() {
        Value::Array(events) => events,
        _ => Vec::new(),
    };
    if let Some(fields) = response["task"].as_object_mut() {
        fields.remove("events");
    }
    let task = &response["task"];
    let total = all.len();
    let skipped = if history {
        0
    } else {
        total.saturating_sub(RECENT_EVENTS)
    };
    let events = &all[skipped..];
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
        format!("body base: {}", string(&task["body_version"])),
        format!(
            "created:   {} by {}",
            string(&task["created_at"]),
            task["agent"].as_str().unwrap_or("unknown")
        ),
    ];
    if task["forwarded"].as_bool().unwrap_or(false) {
        lines.push(format!("from:      {}", string(&task["source_slug"])));
    }
    if let Some(projects) = task["consumer_projects"]
        .as_array()
        .filter(|rows| !rows.is_empty())
    {
        lines.push(format!(
            "consumers: {}",
            projects
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        ));
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
        if skipped > 0 {
            format!(
                "History (latest {} of {total} events; --history shows all):",
                events.len()
            )
        } else {
            "History:".to_owned()
        },
    ]);
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
    let mut data =
        json!({"task": task, "events": events, "events_total": total, "events_omitted": skipped});
    if skipped > 0 {
        data["history_command"] = json!(format!(
            "cj task show {} --history",
            short(string(&task["id"]))
        ));
    }
    output::emit(&data, &lines.join("\n"), json_mode)
}

fn short(id: &str) -> String {
    id.replace('-', "").chars().take(8).collect()
}
fn string(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
