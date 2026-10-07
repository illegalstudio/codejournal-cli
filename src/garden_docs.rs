use crate::{api::Api, git, git_history, git_paths};
use anyhow::Result;
use serde_json::{Value, json};

pub fn check(api: &Api, project_path: &str, docs: &Value) -> Result<Vec<Value>> {
    if api.offline() {
        return Ok(Vec::new());
    }
    let Some(root) = git::root() else {
        return Ok(Vec::new());
    };
    let observations: Vec<_> = docs.as_array().into_iter().flatten().filter_map(|doc| {
        let id = doc["id"].as_str()?;
        let paths: Vec<_> = doc["refs"].as_array().into_iter().flatten()
            .filter(|item| item["kind"] == "path").filter_map(|item| item["value"].as_str())
            .map(|path| git_paths::reference_path(path, &root))
            .filter(|path| git_paths::safe_path(path)).collect();
        Some(json!({"id": id, "changes": git_history::changes_since(doc["updated_at"].as_str(), &paths)}))
    }).collect();
    if observations.is_empty() {
        return Ok(Vec::new());
    }
    let result = api.post_noqueue(
        &format!("{project_path}/garden/doc-checks"),
        &json!({"observations": observations}),
    )?;
    Ok(result["stale_docs"].as_array().cloned().unwrap_or_default())
}
