use super::code_hash;
use crate::{api::Api, git, git_history, git_paths};
use anyhow::{Result, bail};
use serde_json::{Value, json};

pub fn run(api: &Api, base: &str, preview: bool) -> Result<Value> {
    let endpoint = if preview { "preview" } else { "scan" };
    let url = format!("{base}/garden/review/{endpoint}");
    let root = git::root();
    let mut findings = Vec::new();
    let mut reviewed = 0;
    let mut deferred = 0;
    let mut page = 1;
    loop {
        let data = api.get(&format!("{base}/garden?page={page}&review=1"))?;
        if let Some(root) = &root {
            for key in ["entries", "docs"] {
                let rows = data[key].as_array().cloned().unwrap_or_default();
                for batch in rows.chunks(32) {
                    for body in batches(key, batch, root)? {
                        let result = api.post_noqueue(&url, &body)?;
                        if preview {
                            accumulate(&result, &mut findings, &mut reviewed, &mut deferred);
                        }
                    }
                }
            }
        }
        let Some(next) = data["next_page"].as_u64() else {
            break;
        };
        if next <= page {
            bail!("invalid garden pagination");
        }
        page = next;
    }
    let mut result = api.post_noqueue(&url, &json!({"complete": true}))?;
    if preview {
        accumulate(&result, &mut findings, &mut reviewed, &mut deferred);
        findings.sort_by(|a, b| {
            b["priority"]
                .as_i64()
                .cmp(&a["priority"].as_i64())
                .then_with(|| a["id"].as_str().cmp(&b["id"].as_str()))
        });
        findings.dedup_by(|a, b| a["id"] == b["id"]);
        result["counts"] = json!({"pending": findings.len(), "reviewed": reviewed, "deferred": deferred,
            "last_scan": result["progress"]["last_scan"], "last_review": result["progress"]["last_review"]});
        result["findings"] = json!(findings);
        result.as_object_mut().map(|value| value.remove("progress"));
    }
    Ok(result)
}

fn batches(key: &str, rows: &[Value], root: &std::path::Path) -> Result<Vec<Value>> {
    let observations = if key == "entries" {
        git_paths::observe(rows, root)
    } else {
        Vec::new()
    };
    if observations.len() > 1000 && rows.len() > 1 {
        let middle = rows.len() / 2;
        let mut parts = batches(key, &rows[..middle], root)?;
        parts.extend(batches(key, &rows[middle..], root)?);
        return Ok(parts);
    }
    let mut changes = Vec::new();
    for row in rows {
        let paths = row["refs"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|item| item["kind"] == "path")
            .filter_map(|item| item["value"].as_str())
            .map(|path| git_paths::reference_path(path, root))
            .filter(|path| {
                git_paths::safe_path(path) && (key != "entries" || root.join(path).exists())
            })
            .collect::<Vec<_>>();
        let timestamp = if key == "entries" {
            "created_at"
        } else {
            "updated_at"
        };
        let mut observation = json!({"id": row["id"], "changes": git_history::changes_since(row[timestamp].as_str(), &paths),
            "code_hash": code_hash::record(row, root)?});
        if let Some(hash) = row["content_hash"].as_str() {
            observation["content_hash"] = json!(hash);
        }
        changes.push(observation);
    }
    let mut body = json!({"observations": observations});
    body[key] = json!(changes);
    Ok(vec![body])
}

fn accumulate(result: &Value, findings: &mut Vec<Value>, reviewed: &mut u64, deferred: &mut u64) {
    findings.extend(result["findings"].as_array().into_iter().flatten().cloned());
    *reviewed += result["reviewed"].as_u64().unwrap_or(0);
    *deferred += result["deferred"].as_u64().unwrap_or(0);
}
