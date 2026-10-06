use crate::{api::Api, garden_docs, garden_format, output, project_bootstrap, staleness};
use anyhow::{Result, bail};
use serde_json::{Value, json};

pub fn run(
    api: &Api,
    tenant: &str,
    project: Option<&str>,
    dry_run: bool,
    json_mode: bool,
) -> Result<()> {
    if api.offline() && !dry_run {
        bail!("garden applies fixes and needs a writable journal; use --dry-run offline");
    }
    let slug = project_bootstrap::resolved_slug(api, tenant, project)?;
    let path = format!("/api/v1/tenants/{tenant}/projects/{slug}");
    let mut payload = api.get(&format!("{path}/garden/report"))?;
    let mut stale_docs = Vec::new();
    let mut entries = Vec::new();
    let mut page = 1_u64;
    loop {
        let mut result = api.get(&format!("{path}/garden?page={page}"))?;
        staleness::enrich(api, &path, &mut result)?;
        stale_docs.extend(garden_docs::check(api, &path, &result["docs"])?);
        entries.extend(result["entries"].as_array().cloned().unwrap_or_default());
        let Some(next) = result["next_page"].as_u64() else {
            break;
        };
        if next <= page {
            bail!("invalid garden pagination")
        }
        page = next;
    }
    let stale: Vec<_> = entries
        .iter()
        .filter(|entry| entry["staleness"]["stale"] == true)
        .cloned()
        .collect();
    let elsewhere: Vec<_> = entries
        .iter()
        .filter(|entry| entry["staleness"]["stale"] != true && has_refs(entry, "elsewhere"))
        .cloned()
        .collect();
    let applied = if dry_run {
        Vec::new()
    } else {
        let result = api.post(&format!("{path}/garden/maintenance"), &json!({}))?;
        if result["topic_analysis"]["truncated"] == true {
            payload["topic_analysis"] = result["topic_analysis"].clone();
        }
        result["applied"].as_array().cloned().unwrap_or_default()
    };
    payload["project"] = json!(slug);
    payload["dry_run"] = json!(dry_run);
    payload["applied"] = json!(applied);
    payload["stale_entries"] = json!(stale);
    payload["other_branch_entries"] = json!(elsewhere);
    payload["stale_docs"] = json!(stale_docs);
    output::emit(&payload, &garden_format::render(&payload), json_mode)
}

fn has_refs(entry: &Value, kind: &str) -> bool {
    entry["staleness"][kind]
        .as_array()
        .is_some_and(|items| !items.is_empty())
}
