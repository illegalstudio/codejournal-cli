use crate::api::Api;
use crate::output;
use anyhow::Result;
use serde_json::Value;

fn root(tenant: &str) -> String {
    format!("/api/v1/tenants/{tenant}/entries")
}

pub fn show(api: &Api, tenant: &str, id: &str, no_track: bool, json_mode: bool) -> Result<()> {
    let query = if no_track { "?track=0" } else { "" };
    let response = api.get(&format!("{}/{}{}", root(tenant), id, query))?;
    let entry = &response["entry"];
    let topics = array_text(&entry["topics"]);
    let refs = entry["refs"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|item| format!("{}:{}", string(&item["kind"]), string(&item["value"])))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let mut lines = vec![
        format!("id:        {}", string(&entry["id"]).replace('-', "")),
        format!("project:   {}", string(&entry["project_slug"])),
        format!("kind:      {}", string(&entry["kind"])),
        format!("status:    {}", string(&entry["status"])),
        format!(
            "created:   {} by {} on {}",
            string(&entry["created_at"]),
            fallback(&entry["agent"]),
            fallback(&entry["host"])
        ),
        format!("topics:    {}", empty_label(&topics)),
        format!("refs:      {}", empty_label(&refs)),
        format!("scope:     {}", string(&entry["scope"])),
        format!(
            "usage:     {} views, {} helpful, {} reported wrong",
            number(&entry["usage"]["views"]),
            number(&entry["usage"]["helpful"]),
            number(&entry["usage"]["wrong"])
        ),
    ];
    if let Some(notes) = entry["wrong_notes"].as_array() {
        lines.extend(
            notes
                .iter()
                .map(|note| format!("WRONG:     {}", string(note))),
        );
    }
    lines.extend([
        "".to_owned(),
        string(&entry["title"]).to_owned(),
        "".to_owned(),
        string(&entry["body"]).to_owned(),
    ]);
    output::emit(&response, &lines.join("\n"), json_mode)
}

fn array_text(value: &Value) -> String {
    value
        .as_array()
        .map(|items| items.iter().map(string).collect::<Vec<_>>().join(", "))
        .unwrap_or_default()
}

fn string(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}

fn number(value: &Value) -> u64 {
    value.as_u64().unwrap_or(0)
}

fn fallback(value: &Value) -> &str {
    value.as_str().unwrap_or("unknown")
}

fn empty_label(value: &str) -> &str {
    if value.is_empty() { "(none)" } else { value }
}
