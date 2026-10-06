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
    before_revision: Option<u32>,
    body_only: bool,
    current_only: bool,
    json_mode: bool,
) -> Result<()> {
    let mut params = Vec::new();
    if let Some(number) = revision {
        params.push(("revision", number.to_string()));
    }
    if current_only || body_only || (!history && !json_mode) {
        params.push(("history", "0".to_owned()));
    } else if history {
        params.push(("history_content", "0".to_owned()));
    } else {
        params.push(("history_content", "1".to_owned()));
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
    if history {
        if let Some(before) = result["revision_next"].as_u64() {
            lines.push(format!(
                "More revisions: cj {} show {id} --history --before-revision {before}",
                noun(kind)
            ));
        }
    }
    output::emit(&result, &lines.join("\n"), json_mode)
}
