use crate::{
    api::Api, garden_format, garden_review, output, project_bootstrap, staleness, topic_similarity,
};
use anyhow::{Context, Result, bail};
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
    let topics = api.get(&format!("/api/v1/tenants/{tenant}/topics"))?;
    let (certain, possible) = topic_similarity::groups(&topics["topics"]);
    let maintenance = api.get(&format!("{path}/garden/maintenance"))?;
    let mut entries = Vec::new();
    let mut page = 1_u64;
    loop {
        let mut result = api.get(&format!("{path}/garden?page={page}"))?;
        staleness::enrich(api, &path, &mut result)?;
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
    let export = api.get(&format!("/api/v1/tenants/{tenant}/export?project={slug}"))?;
    let records = export["records"]
        .as_array()
        .context("invalid garden export")?;
    let review = garden_review::collect(records, &entries);
    let applied = if dry_run {
        Vec::new()
    } else {
        let result = api.post(
            &format!("{path}/garden/maintenance"),
            &json!({"topic_groups": certain}),
        )?;
        result["applied"].as_array().cloned().unwrap_or_default()
    };
    let payload = json!({"project": slug, "dry_run": dry_run, "applied": applied,
        "secrets": maintenance["secrets"], "topic_groups": certain,
        "possible_topics": possible, "topic_counts": topics["topics"], "duplicates": review.duplicates,
        "stale_entries": stale, "other_branch_entries": elsewhere,
        "stale_docs": review.stale_docs, "idle_plans": review.idle_plans,
        "reported_wrong": maintenance["reported_wrong"], "doc_candidates": review.doc_candidates});
    output::emit(&payload, &garden_format::render(&payload), json_mode)
}

fn has_refs(entry: &Value, kind: &str) -> bool {
    entry["staleness"][kind]
        .as_array()
        .is_some_and(|items| !items.is_empty())
}
