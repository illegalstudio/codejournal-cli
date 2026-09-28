use crate::api::Api;
use crate::git;
use anyhow::Result;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub fn enrich(api: &Api, project_path: &str, result: &mut Value) -> Result<()> {
    let Some(entries) = result["entries"].as_array() else {
        return Ok(());
    };
    let Some(root) = git::root() else {
        return Ok(());
    };
    let mut paths: BTreeMap<String, (BTreeSet<String>, BTreeSet<String>)> = BTreeMap::new();
    let ids: Vec<String> = entries
        .iter()
        .filter_map(|entry| entry["id"].as_str().map(str::to_owned))
        .collect();
    for entry in entries {
        let Some(refs) = entry["refs"].as_array() else {
            continue;
        };
        let entry_paths: Vec<&str> = refs
            .iter()
            .filter(|item| item["kind"] == "path")
            .filter_map(|item| item["value"].as_str())
            .collect();
        for path in entry_paths {
            let candidates = paths.entry(path.to_owned()).or_default();
            for item in refs {
                if let Some(value) = item["value"].as_str() {
                    if item["kind"] == "branch" {
                        candidates.0.insert(value.to_owned());
                    }
                    if item["kind"] == "commit" {
                        candidates.1.insert(value.to_owned());
                    }
                }
            }
        }
    }
    if paths.is_empty() {
        return Ok(());
    }
    let default = default_branch();
    let observations: Vec<Value> = paths
        .iter()
        .filter_map(|(path, (branches, commits))| {
            if !safe_path(path) {
                return None;
            }
            let present = root.join(path).symlink_metadata().is_ok();
            let branches: Vec<&String> = branches
                .iter()
                .filter(|branch| safe_branch(branch) && object_has_path(branch, path))
                .collect();
            let commits: Vec<&String> = commits
                .iter()
                .filter(|commit| safe_commit(commit) && object_has_path(commit, path))
                .collect();
            let default = default.as_deref().is_some_and(|branch| {
                object_has_path(branch, path) || object_has_path(&format!("origin/{branch}"), path)
            });
            Some(
                json!({"path": path, "present": present, "branches": branches,
            "commits": commits, "default": default}),
            )
        })
        .collect();
    let response = api.post(
        &format!("{project_path}/path-checks"),
        &json!({"ids": ids, "observations": observations}),
    )?;
    let checks = response["checks"]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("invalid path checks"))?;
    if let Some(entries) = result["entries"].as_array_mut() {
        for entry in entries {
            if let Some(id) = entry["id"].as_str() {
                if let Some(check) = checks.get(id) {
                    entry["staleness"] = check.clone();
                }
            }
        }
    }
    Ok(())
}

fn safe_path(path: &str) -> bool {
    !Path::new(path).is_absolute()
        && !path.is_empty()
        && !Path::new(path)
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
}

fn safe_branch(branch: &str) -> bool {
    !branch.starts_with('-')
        && !branch.contains(':')
        && !branch.contains("..")
        && git::bytes(&["check-ref-format", "--branch", branch]).is_some()
}

fn safe_commit(commit: &str) -> bool {
    (7..=64).contains(&commit.len()) && commit.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn object_has_path(revision: &str, path: &str) -> bool {
    git::bytes(&["cat-file", "-e", &format!("{revision}:{path}")]).is_some()
}

pub fn default_branch() -> Option<String> {
    git::output(&[
        "symbolic-ref",
        "--quiet",
        "--short",
        "refs/remotes/origin/HEAD",
    ])
    .map(|branch| short_default(&branch))
    .or_else(|| {
        ["main", "master"]
            .into_iter()
            .find(|branch| {
                git::bytes(&["rev-parse", "--verify", &format!("refs/heads/{branch}")]).is_some()
            })
            .map(str::to_owned)
    })
}

fn short_default(branch: &str) -> String {
    branch.strip_prefix("origin/").unwrap_or(branch).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_head_matches_the_local_default_branch() {
        assert_eq!(short_default("origin/main"), "main");
    }
}
