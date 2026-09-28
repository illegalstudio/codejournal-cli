use crate::{api::Api, api_cache, git};
use anyhow::Result;
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub fn load(api: &Api, base: &str, cache_path: &str, mut body: Value) -> Result<Value> {
    let shared_cache = format!("{base}/brief-cache?limit={}&pinned_limit={}&log_limit={}",
        body["limit"], body["pinned_limit"], body["log_limit"]);
    if api.offline() {
        return api_cache::read(api.server(), &api.token, &shared_cache)
            .or_else(|_| api.get(cache_path));
    }
    if let Some(changes) = changed_files() {
        if changes["files"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
        {
            body["files"] = changes["files"].clone();
            body["branch"] = changes["branch"].clone();
            body["compared_to"] = changes["compared_to"].clone();
        }
    }
    match api.post_noqueue(&format!("{base}/brief"), &body) {
        Ok(result) => {
            let _ = api_cache::write(api.server(), &api.token, cache_path, &result);
            let _ = api_cache::write(api.server(), &api.token, &shared_cache, &result);
            Ok(result)
        }
        Err(_) => api_cache::read(api.server(), &api.token, &shared_cache)
            .or_else(|_| api.get(cache_path)),
    }
}

pub fn changed_files() -> Option<Value> {
    git::root()?;
    let branch = git::output(&["symbolic-ref", "--quiet", "--short", "HEAD"]);
    let base = ["origin/main", "origin/master", "main", "master"]
        .into_iter()
        .find(|candidate| git::output(&["rev-parse", "--verify", candidate]).is_some())
        .map(str::to_owned);
    let mut files = BTreeSet::new();
    let mut compared = None;
    if let (Some(branch), Some(base)) = (&branch, &base) {
        if branch != base.trim_start_matches("origin/") {
            if let Some(merge_base) = git::output(&["merge-base", "HEAD", base]) {
                compared = Some(base.clone());
                add_lines(
                    &mut files,
                    git::output(&["diff", "--name-only", &merge_base, "HEAD"]),
                );
            }
        }
    }
    add_lines(&mut files, git::output(&["diff", "--name-only", "HEAD"]));
    add_lines(
        &mut files,
        git::output(&["ls-files", "--others", "--exclude-standard"]),
    );
    Some(json!({"branch": branch, "compared_to": compared,
        "files": files.into_iter().take(1000).collect::<Vec<_>>()}))
}

fn add_lines(files: &mut BTreeSet<String>, text: Option<String>) {
    for line in text.unwrap_or_default().lines() {
        if !line.is_empty() {
            files.insert(line.to_owned());
        }
    }
}
