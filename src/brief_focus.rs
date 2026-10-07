use crate::{
    api::Api, api_cache, brief_manifests, git, outbox, project_bootstrap, project_provides,
    request_outbox,
};
use anyhow::Result;
use serde_json::{Value, json};
use std::collections::BTreeSet;

mod discovery;

pub fn load(api: &Api, base: &str, cache_path: &str, mut body: Value) -> Result<Value> {
    let canonical = project_bootstrap::read_path(api, base)?;
    let cache_path = cache_path.replacen(base, &canonical, 1);
    let base = canonical.as_str();
    let audit = body["audit"] == true;
    let shared_cache = format!(
        "{base}/brief-cache?limit={}&pinned_limit={}&log_limit={}&visibility={}&summary={}{}",
        body["limit"],
        body["pinned_limit"],
        body["log_limit"],
        body["visibility"].as_str().unwrap_or("active"),
        body["summary"] == true,
        if audit { "&audit=1" } else { "" }
    );
    if api.offline() {
        return api_cache::read(api.server(), &api.token, &shared_cache)
            .or_else(|_| api.get(&cache_path))
            .map(|result| localize(result, "offline", &body));
    }
    if !audit && let Some(changes) = changed_files() {
        if changes["files"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
        {
            body["files"] = changes["files"].clone();
            body["branch"] = changes["branch"].clone();
            body["compared_to"] = changes["compared_to"].clone();
        }
    }
    if !audit && let Some(manifests) = brief_manifests::fingerprints() {
        body["manifest_names"] = manifests;
    }
    if !audit && git::root().is_some() {
        body["provides_auto"] = json!(project_provides::detect());
    }
    match api.post_noqueue(&format!("{base}/brief"), &body) {
        Ok(result) => {
            let _ = api_cache::write(api.server(), &api.token, &cache_path, &result);
            let _ = api_cache::write(api.server(), &api.token, &shared_cache, &result);
            Ok(localize(result, "remote", &body))
        }
        Err(error) if error.to_string().starts_with("API returned 4") => Err(error),
        Err(_) => api_cache::read(api.server(), &api.token, &shared_cache)
            .or_else(|_| api.get(&cache_path))
            .map(|result| localize(result, "offline", &body)),
    }
}

fn localize(mut result: Value, mode: &str, policy: &Value) -> Value {
    discovery::apply(
        &mut result,
        policy["visibility"] != "all",
        policy["summary"] == true,
    );
    result["mode"] = Value::String(mode.to_owned());
    result["pending_outbox"] =
        Value::from(outbox::pending().unwrap_or(0) + request_outbox::pending().unwrap_or(0));
    result
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
