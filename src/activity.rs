use crate::api::Api;
use crate::{git, output, project};
use anyhow::{Context, Result};
use clap::Subcommand;
use serde_json::json;
use std::fs;
use std::time::UNIX_EPOCH;

#[derive(Subcommand)]
pub enum ActivityAction {
    Publish,
}

pub fn publish(api: &Api, tenant: &str, project_name: Option<&str>) -> Result<()> {
    let root = git::root().context("checkout activity requires a Git repository")?;
    let status = git::bytes(&["status", "--porcelain=v1", "-z", "--untracked-files=all"])
        .context("cannot inspect Git status")?;
    let paths = status_paths(&status);
    let latest_file = paths
        .iter()
        .filter_map(|path| fs::metadata(root.join(path)).ok())
        .filter_map(|meta| meta.modified().ok())
        .filter_map(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .max();
    let commit_epoch =
        git::output(&["log", "-1", "--format=%ct"]).and_then(|value| value.parse::<u64>().ok());
    let touched_epoch = latest_file.into_iter().chain(commit_epoch).max();
    let host = std::env::var("HOSTNAME")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            std::process::Command::new("hostname")
                .output()
                .ok()
                .map(|value| String::from_utf8_lossy(&value.stdout).trim().to_owned())
        })
        .context("cannot determine host name")?;
    let branch = git::output(&["symbolic-ref", "--quiet", "--short", "HEAD"]);
    let commit = git::output(&["rev-parse", "HEAD"]);
    let kind = if git::output(&["rev-parse", "--git-dir"])
        != git::output(&["rev-parse", "--git-common-dir"])
    {
        "worktree"
    } else {
        "main"
    };
    let endpoint = format!(
        "/api/v1/tenants/{tenant}/projects/{}/checkout-activity",
        project::slug(project_name)?
    );
    output::json(&crate::activity_delivery::publish(
        api,
        &endpoint,
        &json!({
            "host": host, "path": root.to_string_lossy(), "kind": kind,
            "branch": branch, "commit_sha": commit,
            "commit_epoch": commit_epoch,
            "subject": git::output(&["log", "-1", "--format=%s"]),
            "dirty": paths.len(), "touched_epoch": touched_epoch,
        }),
    )?)
}

fn status_paths(bytes: &[u8]) -> Vec<String> {
    let mut paths = Vec::new();
    let mut items = bytes
        .split(|byte| *byte == 0)
        .filter(|item| !item.is_empty());
    while let Some(item) = items.next() {
        if item.len() < 4 {
            continue;
        }
        paths.push(String::from_utf8_lossy(&item[3..]).to_string());
        if item[0] == b'R' || item[1] == b'R' || item[0] == b'C' || item[1] == b'C' {
            items.next();
        }
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn porcelain_rename_counts_one_dirty_path() {
        assert_eq!(status_paths(b"R  new.rs\0old.rs\0?? added.rs\0").len(), 2);
    }
}
