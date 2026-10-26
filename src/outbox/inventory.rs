use crate::output;
use anyhow::Result;
use serde_json::json;

pub fn list(body: bool, json_mode: bool) -> Result<()> {
    let events = super::entries()?.into_iter().map(|(_, event)| {
        let bound = super::origin::bound(&event).is_some();
        let mut item = json!({"id": event["id"], "type": event["type"],
            "project": event["project"], "agent": event["agent"], "ts": event["ts"],
            "binding": if bound { "bound" } else { "unbound" },
            "reason": if bound { json!(null) } else { event.get("origin_unavailable_reason").cloned().unwrap_or(json!("unknown")) }});
        if body { item["body"] = crate::api::sanitized(event); }
        item
    }).collect::<Vec<_>>();
    let text = events
        .iter()
        .map(|item| {
            let mut text = format!(
                "{}  {}  {}  {}  {}{}",
                item["id"].as_str().unwrap_or("?"),
                item["binding"].as_str().unwrap_or("?"),
                item["type"].as_str().unwrap_or("?"),
                item["project"].as_str().unwrap_or("?"),
                item["ts"].as_str().unwrap_or("?"),
                item["reason"]
                    .as_str()
                    .map(|reason| format!(" ({reason})"))
                    .unwrap_or_default()
            );
            if body {
                text.push_str(&format!(
                    "\n{}",
                    serde_json::to_string_pretty(&item["body"]).unwrap_or_default()
                ));
            }
            text
        })
        .collect::<Vec<_>>()
        .join("\n");
    output::emit(&json!({"hook_events": events}), &text, json_mode)
}
