use super::code_hash;
use crate::{git_history, git_paths};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::path::Path;

const MAX_RECORDS: usize = 250;
const MAX_PATHS: usize = 1000;
const MAX_CANDIDATES: usize = 30;
const MAX_BYTES: usize = 4 * 1024 * 1024 - 64;

pub fn page(data: &Value, root: &Path) -> Result<Vec<Value>> {
    let entries = data["entries"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    let docs = data["docs"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    batches(entries, docs, root)
}

fn batches(entries: &[Value], docs: &[Value], root: &Path) -> Result<Vec<Value>> {
    if entries.is_empty() && docs.is_empty() {
        return Ok(Vec::new());
    }
    if entries.len() > MAX_RECORDS || docs.len() > MAX_RECORDS {
        return split(entries, docs, root);
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
        return split(entries, docs, root);
    }
    let body = json!({"observations": paths, "entries": changes(entries, root, true)?,
        "docs": changes(docs, root, false)?});
    if serde_json::to_vec(&body)?.len() > MAX_BYTES {
        return split(entries, docs, root);
    }
    Ok(vec![body])
}

fn split(entries: &[Value], docs: &[Value], root: &Path) -> Result<Vec<Value>> {
    ensure!(
        entries.len() + docs.len() > 1,
        "Garden observation exceeds supported request limits"
    );
    let middle = (entries.len() + docs.len()) / 2;
    let entry_mid = entries.len().min(middle);
    let doc_mid = middle - entry_mid;
    let mut parts = batches(&entries[..entry_mid], &docs[..doc_mid], root)?;
    parts.extend(batches(&entries[entry_mid..], &docs[doc_mid..], root)?);
    Ok(parts)
}

fn changes(rows: &[Value], root: &Path, entry: bool) -> Result<Vec<Value>> {
    rows.iter()
        .map(|row| {
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
            "code_hash": code_hash::record(row, root)?});
            if let Some(hash) = row["content_hash"].as_str() {
                observation["content_hash"] = json!(hash);
            }
            Ok(observation)
        })
        .collect()
}
