use crate::api::Api;
use crate::{output, plan_history};
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
    all_revisions: bool,
    before_revision: Option<u32>,
    body_only: bool,
    json_mode: bool,
) -> Result<()> {
    let mut params = Vec::new();
    if let Some(number) = revision {
        params.push(("revision", number.to_string()));
    }
    if history {
        params.push(("history_content", "0".to_owned()));
    } else {
        params.push(("history", if all_revisions { "1" } else { "0" }.to_owned()));
    }
    if let Some(before) = before_revision {
        params.push(("history_before", before.to_string()));
    }
    let url = reqwest::Url::parse_with_params("http://local/", &params)?;
    let query = url
        .query()
        .map(|query| format!("?{query}"))
        .unwrap_or_default();
    let result = api.get(&format!("/api/v1/tenants/{tenant}/{kind}/{id}{query}"))?;
    let item = &result[noun(kind)];
    if body_only && !json_mode {
        crate::stdout::println!("{}", item["body"].as_str().context("missing body")?);
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
    ];
    if let Some(count) = result["revision_count"].as_u64() {
        lines.push(format!(
            "revisions: {count} (current {})",
            result["current_revision"]
        ));
    }
    lines.extend([
        "".to_owned(),
        text(&item["title"]).to_owned(),
        "".to_owned(),
        text(&item["body"]).to_owned(),
    ]);
    if history || all_revisions {
        plan_history::append(&mut lines, &result, noun(kind), id, all_revisions);
    }
    output::emit(&result, &lines.join("\n"), json_mode)
}
