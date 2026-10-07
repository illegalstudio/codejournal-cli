use crate::{api::Api, git, git_history, git_paths};
use anyhow::Result;
use serde_json::{Value, json};

pub use git_paths::default_branch;

pub fn enrich(api: &Api, project_path: &str, result: &mut Value) -> Result<()> {
    if api.offline() {
        return Ok(());
    }
    let Some(entries) = result["entries"].as_array().cloned() else {
        return Ok(());
    };
    let Some(root) = git::root() else {
        return Ok(());
    };
    for batch in entries.chunks(32) {
        let checks = check_batch(api, project_path, batch, &root)?;
        if let Some(rows) = result["entries"].as_array_mut() {
            for entry in rows {
                if let Some(check) = entry["id"].as_str().and_then(|id| checks.get(id)) {
                    entry["staleness"] = check.clone();
                }
            }
        }
    }
    Ok(())
}

fn check_batch(
    api: &Api,
    project_path: &str,
    entries: &[Value],
    root: &std::path::Path,
) -> Result<serde_json::Map<String, Value>> {
    let observations = git_paths::observe(entries, root);
    if observations.is_empty() {
        return Ok(serde_json::Map::new());
    }
    let changes: Vec<_> = entries.iter().filter_map(|entry| {
        let id = entry["id"].as_str()?;
        let present: Vec<_> = entry["refs"].as_array().into_iter().flatten()
            .filter(|item| item["kind"] == "path")
            .filter_map(|item| item["value"].as_str())
            .map(|path| git_paths::reference_path(path, root))
            .filter(|path| git_paths::safe_path(path) && root.join(path).exists()).collect();
        Some(json!({"id": id, "changes": git_history::changes_since(entry["created_at"].as_str(), &present)}))
    }).collect();
    let ids: Vec<_> = entries
        .iter()
        .filter_map(|entry| entry["id"].as_str())
        .collect();
    let Ok(response) = api.post_noqueue(
        &format!("{project_path}/path-checks"),
        &json!({"ids": ids, "observations": observations, "changes": changes}),
    ) else {
        return Ok(serde_json::Map::new());
    };
    response["checks"]
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("invalid path checks"))
}
