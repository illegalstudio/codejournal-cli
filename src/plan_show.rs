use crate::api::Api;
use crate::output;
use anyhow::{Context, Result};
use serde_json::Value;

fn noun(kind: &str) -> &str {
    if kind == "docs" { "doc" } else { "plan" }
}
fn text(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}

pub fn show(
    api: &Api,
    tenant: &str,
    kind: &str,
    id: &str,
    revision: Option<u32>,
    history: bool,
    body_only: bool,
    current_only: bool,
    json_mode: bool,
) -> Result<()> {
    let query = revision
        .map(|number| format!("?revision={number}"))
        .unwrap_or_default();
    let query = if current_only {
        format!(
            "{query}{}history=0",
            if query.is_empty() { "?" } else { "&" }
        )
    } else {
        query
    };
    let result = api.get(&format!("/api/v1/tenants/{tenant}/{kind}/{id}{query}"))?;
    let item = &result[noun(kind)];
    if body_only && !json_mode {
        println!("{}", item["body"].as_str().context("missing body")?);
        return Ok(());
    }
    let mut lines = vec![
        format!("id:        {}", text(&item["id"]).replace('-', "")),
        format!(
            "project:   {}",
            if item["scope"] == "global" {
                "GLOBAL"
            } else {
                text(&item["project_slug"])
            }
        ),
        format!("status:    {}", text(&item["status"])),
        format!("revision:  {}", item["revision"]),
        format!("updated:   {}", text(&item["updated_at"])),
        "".to_owned(),
        text(&item["title"]).to_owned(),
        "".to_owned(),
        text(&item["body"]).to_owned(),
    ];
    if history {
        lines.push("".to_owned());
        lines.push("History:".to_owned());
        for record in result["revisions"].as_array().into_iter().flatten() {
            lines.push(format!(
                "  rev {} {} {}",
                record["revision"],
                text(&record["created_at"]),
                text(&record["note"])
            ));
        }
    }
    output::emit(&result, &lines.join("\n"), json_mode)
}
