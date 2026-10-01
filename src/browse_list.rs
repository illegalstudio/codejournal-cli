use crate::api::Api;
use anyhow::{Result, bail};
use serde_json::Value;

pub fn entries(
    api: &Api,
    tenant: &str,
    slug: Option<&str>,
    all_statuses: bool,
) -> Result<Vec<String>> {
    let mut lines = Vec::new();
    let mut page = 1;
    loop {
        let scope = slug
            .map(|value| format!("&project={value}"))
            .unwrap_or_default();
        let status = if all_statuses { "all" } else { "active" };
        let result = api.get(&format!(
            "/api/v1/tenants/{tenant}/entries?limit=100&page={page}&status={status}{scope}"
        ))?;
        for entry in result["entries"].as_array().into_iter().flatten() {
            let id = value(&entry["id"]);
            let project = slug.unwrap_or_else(|| value(&entry["project_slug"]));
            let kind = value(&entry["kind"]);
            let created = value(&entry["created_at"]);
            let date = &created[..created.len().min(10)];
            let title = value(&entry["title"]).replace(['\t', '\n'], " ");
            lines.push(format!("{id}\t{project}\t{kind:<12}\t{date}\t{title}"));
        }
        let Some(next) = result["next_page"].as_u64() else {
            break;
        };
        if next <= page || next > 10000 {
            bail!("invalid entry pagination");
        }
        page = next;
    }
    Ok(lines)
}

pub fn projects(api: &Api, tenant: &str) -> Result<()> {
    let result = api.get(&format!("/api/v1/tenants/{tenant}/projects"))?;
    for row in result["projects"].as_array().into_iter().flatten() {
        crate::stdout::println!(
            "{}\t{}\t{} active",
            value(&row["slug"]),
            value(&row["name"]),
            row["active_entries"].as_u64().unwrap_or(0)
        );
    }
    Ok(())
}

fn value(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
