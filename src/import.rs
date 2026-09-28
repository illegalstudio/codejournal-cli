use crate::{api::Api, output};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub fn run(api: &Api, tenant: &str, file: PathBuf, json_mode: bool) -> Result<()> {
    if api.offline() {
        bail!("import requires a writable journal; the remote is unreachable");
    }
    let text = fs::read_to_string(&file)?;
    let mut records = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str::<Value>)
        .collect::<serde_json::Result<Vec<_>>>()?;
    for record in &mut records {
        canonicalize(record)?;
    }
    let mut ids = HashMap::new();
    for row in &records {
        if row["type"] == "project" {
            let id = row["id"].as_str().context("project ID missing")?;
            let slug = row["slug"].as_str().context("project slug missing")?;
            ids.insert(id.to_owned(), slug.to_owned());
        }
    }
    records.sort_by_key(|row| rank(row["type"].as_str().unwrap_or("")));
    if records
        .iter()
        .any(|row| rank(row["type"].as_str().unwrap_or("")) == 255)
    {
        bail!("import file contains an unsupported record type");
    }
    let path = format!("/api/v1/tenants/{tenant}/import");
    let mut aliases = HashMap::<String, String>::new();
    let mut counts = json!({"projects": 0, "entries": 0, "logs": 0, "plans": 0,
        "notifications": 0, "tasks": 0, "feedback": 0, "skipped": 0});
    for chunk in records.chunks(100) {
        let batch = chunk
            .iter()
            .map(|row| annotate(row, &ids, &aliases))
            .collect::<Result<Vec<_>>>()?;
        let response = api.post_noqueue(&path, &json!({"records": batch}))?;
        for (name, value) in response["counts"].as_object().into_iter().flatten() {
            counts[name] = json!(counts[name].as_u64().unwrap_or(0) + value.as_u64().unwrap_or(0));
        }
        for (source, target) in response["project_map"].as_object().into_iter().flatten() {
            if let Some(target) = target.as_str() {
                aliases.insert(source.to_owned(), target.to_owned());
            }
        }
    }
    let links = records
        .iter()
        .filter_map(|row| {
            if row["type"] == "entry" && row["superseded_by"].as_str().is_some() {
                Some(json!({"id": row["id"], "superseded_by": row["superseded_by"]}))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    for chunk in links.chunks(100) {
        api.post_noqueue(&format!("{path}/resolve"), &json!({"links": chunk}))?;
    }
    let message = format!(
        "Imported {} entries across {} projects ({} skipped).",
        counts["entries"], counts["projects"], counts["skipped"]
    );
    output::emit(&counts, &message, json_mode)
}

fn annotate(
    row: &Value,
    ids: &HashMap<String, String>,
    aliases: &HashMap<String, String>,
) -> Result<Value> {
    let mut value = row.clone();
    for (id_field, slug_field) in [
        ("project_id", "project_slug"),
        ("source_project_id", "source_project_slug"),
    ] {
        if let Some(id) = row[id_field].as_str() {
            if id_field == "source_project_id" && !ids.contains_key(id) {
                continue;
            }
            let slug = ids
                .get(id)
                .with_context(|| format!("import project {id} is missing"))?;
            value[slug_field] = json!(aliases.get(slug).unwrap_or(slug));
        }
    }
    Ok(value)
}

fn rank(kind: &str) -> u8 {
    match kind {
        "project" => 0,
        "entry" => 1,
        "plan" => 2,
        "log" => 3,
        "notification" => 4,
        "task" => 5,
        "feedback" => 6,
        _ => 255,
    }
}

fn canonicalize(record: &mut Value) -> Result<()> {
    for key in [
        "id",
        "project_id",
        "plan_id",
        "source_project_id",
        "source_entry_id",
        "superseded_by",
        "watch_id",
    ] {
        canonical_field(record, key)?;
    }
    for nested in ["revisions", "events"] {
        if let Some(rows) = record[nested].as_array_mut() {
            for row in rows {
                for key in ["id", "plan_id", "task_id"] {
                    canonical_field(row, key)?;
                }
            }
        }
    }
    Ok(())
}

fn canonical_field(record: &mut Value, key: &str) -> Result<()> {
    if let Some(raw) = record[key].as_str() {
        record[key] = json!(
            uuid::Uuid::parse_str(raw)
                .with_context(|| format!("invalid {key}: {raw}"))?
                .hyphenated()
                .to_string()
        );
    }
    Ok(())
}
