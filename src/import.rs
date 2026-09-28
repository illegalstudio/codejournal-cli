use crate::{api::Api, output};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
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
    let path = format!(
        "/api/v1/tenants/{tenant}/import/batches/{}",
        batch_id(&text)
    );
    let empty = json!({"projects": 0, "entries": 0, "logs": 0, "plans": 0,
        "notifications": 0, "tasks": 0, "feedback": 0, "skipped": 0});
    for (sequence, chunk) in records.chunks(100).enumerate() {
        let batch = chunk
            .iter()
            .map(|row| annotate(row, &ids))
            .collect::<Result<Vec<_>>>()?;
        api.put_noqueue(
            &format!("{path}/chunks/{sequence}"),
            &json!({"records": batch}),
        )?;
    }
    let counts = if records.is_empty() {
        empty
    } else {
        api.post_noqueue(
            &format!("{path}/finalize"),
            &json!({"chunks": records.len().div_ceil(100)}),
        )?["counts"]
            .clone()
    };
    let message = format!(
        "Imported {} entries across {} projects ({} skipped).",
        counts["entries"], counts["projects"], counts["skipped"]
    );
    output::emit(&counts, &message, json_mode)
}

fn annotate(row: &Value, ids: &HashMap<String, String>) -> Result<Value> {
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
            value[slug_field] = json!(slug);
        }
    }
    Ok(value)
}

fn batch_id(text: &str) -> uuid::Uuid {
    let hash = Sha256::digest(text.as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&hash[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(bytes)
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
