use crate::api::Api;
use crate::{output, project};
use anyhow::{Context, Result};
use serde_json::Value;

fn noun(kind: &str) -> &str {
    if kind == "docs" { "doc" } else { "plan" }
}
fn text(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
fn short(id: &str) -> String {
    id.replace('-', "").chars().take(8).collect()
}

pub fn list(
    api: &Api,
    tenant: &str,
    explicit_project: Option<&str>,
    kind: &str,
    status: String,
    grep: Option<String>,
    path: Option<String>,
    all_projects: bool,
    global: bool,
    local: bool,
    json_mode: bool,
) -> Result<()> {
    let base = if all_projects || global {
        format!("/api/v1/tenants/{tenant}/{kind}")
    } else {
        format!(
            "/api/v1/tenants/{tenant}/projects/{}/{kind}",
            project::slug(explicit_project)?
        )
    };
    let mut params = vec![("status", status)];
    if global {
        params.push(("scope", "global".to_owned()));
    } else if kind == "docs" && !all_projects && !local {
        params.push(("include_global", "1".to_owned()));
    }
    if let Some(grep) = grep {
        params.push(("grep", grep));
    }
    if let Some(path) = path {
        params.push(("path", path));
    }
    let query = reqwest::Url::parse_with_params("http://local/", &params)?;
    let result = api.get(&format!("{base}?{}", query.query().unwrap_or("")))?;
    let mut lines = Vec::new();
    for item in result[kind].as_array().into_iter().flatten() {
        let where_text = if all_projects || global || item["scope"] == "global" {
            format!(
                "{:<24} ",
                if item["scope"] == "global" {
                    "GLOBAL"
                } else {
                    text(&item["project_slug"])
                }
            )
        } else {
            String::new()
        };
        let when = text(&item["updated_at"]);
        lines.push(format!(
            "  {}  {:<9} {where_text}{}  (rev {}, updated {})",
            short(text(&item["id"])),
            text(&item["status"]),
            text(&item["title"]),
            item["revision"].as_u64().unwrap_or(0),
            &when[..when.len().min(10)]
        ));
    }
    output::emit(
        &result,
        &if lines.is_empty() {
            format!("  (no {}s)", noun(kind))
        } else {
            lines.join("\n")
        },
        json_mode,
    )
}

pub fn show(
    api: &Api,
    tenant: &str,
    kind: &str,
    id: &str,
    revision: Option<u32>,
    history: bool,
    body_only: bool,
    json_mode: bool,
) -> Result<()> {
    let query = revision
        .map(|number| format!("?revision={number}"))
        .unwrap_or_default();
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
