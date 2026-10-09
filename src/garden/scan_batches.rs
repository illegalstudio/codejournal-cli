use super::code_hash;
use crate::{git_history, git_paths};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::path::Path;

pub struct Page {
    pub batches: Vec<Value>,
    pub failures: Vec<Value>,
}

const MAX_RECORDS: usize = 250;
const MAX_PATHS: usize = 1000;
const MAX_CANDIDATES: usize = 30;
const MAX_BYTES: usize = 4 * 1024 * 1024 - 64;

pub fn page(data: &Value, root: &Path) -> Result<Page> {
    let entries = data["entries"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    let docs = data["docs"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    let mut failures = Vec::new();
    let batches = batches(entries, docs, root, &mut failures)?;
    Ok(Page { batches, failures })
}

fn batches(
    entries: &[Value],
    docs: &[Value],
    root: &Path,
    failures: &mut Vec<Value>,
) -> Result<Vec<Value>> {
    if entries.is_empty() && docs.is_empty() {
        return Ok(Vec::new());
    }
    if entries.len() > MAX_RECORDS || docs.len() > MAX_RECORDS {
        return split(entries, docs, root, failures);
    }
    let paths = git_paths::observe(entries, root);
    if paths.len() > MAX_PATHS
        || paths.iter().any(|row| {
            ["branches", "commits"].iter().any(|key| {
                row[key]
                    .as_array()
                    .is_some_and(|items| items.len() > MAX_CANDIDATES)
            })
        })
    {
        return split(entries, docs, root, failures);
    }
    let body = json!({"observations": paths, "entries": changes(entries, root, true, failures),
        "docs": changes(docs, root, false, failures)});
    if serde_json::to_vec(&body)?.len() > MAX_BYTES {
        return split(entries, docs, root, failures);
    }
    Ok(vec![body])
}

fn split(
    entries: &[Value],
    docs: &[Value],
    root: &Path,
    failures: &mut Vec<Value>,
) -> Result<Vec<Value>> {
    ensure!(
        entries.len() + docs.len() > 1,
        "Garden observation exceeds supported request limits for record {}",
        entries
            .first()
            .or_else(|| docs.first())
            .map_or(&Value::Null, |row| &row["id"])
    );
    let middle = (entries.len() + docs.len()) / 2;
    let entry_mid = entries.len().min(middle);
    let doc_mid = middle - entry_mid;
    let mut parts = batches(&entries[..entry_mid], &docs[..doc_mid], root, failures)?;
    parts.extend(batches(
        &entries[entry_mid..],
        &docs[doc_mid..],
        root,
        failures,
    )?);
    Ok(parts)
}

fn changes(rows: &[Value], root: &Path, entry: bool, failures: &mut Vec<Value>) -> Vec<Value> {
    rows.iter()
        .filter_map(|row| {
            if failures.iter().any(|failure| failure["id"] == row["id"]) {
                return None;
            }
            let hash = match code_hash::record(row, root) {
                Ok(hash) => hash,
                Err(error) => {
                    let message = crate::secret_redaction::text(&format!("{error:#}")).0;
                    failures.push(
                        json!({"id": row["id"], "kind": if entry {"entry"} else {"doc"},
                        "error": message.chars().take(500).collect::<String>()}),
                    );
                    return None;
                }
            };
            let paths = row["refs"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|item| item["kind"] == "path")
                .filter_map(|item| item["value"].as_str())
                .map(|path| git_paths::reference_path(path, root))
                .filter(|path| git_paths::safe_path(path) && (!entry || root.join(path).exists()))
                .collect::<Vec<_>>();
            let timestamp = if entry { "created_at" } else { "updated_at" };
            let mut observation = json!({"id": row["id"],
            "changes": git_history::changes_since(row[timestamp].as_str(), &paths),
            "code_hash": hash});
            if let Some(hash) = row["content_hash"].as_str() {
                observation["content_hash"] = json!(hash);
            }
            Some(observation)
        })
        .collect()
}
