use crate::api::Api;
use crate::{commands, output, refs, session_git, session_state};
use anyhow::Result;
use serde_json::json;

pub fn add(
    api: &Api,
    tenant: &str,
    project: Option<&str>,
    title: &str,
    body: &str,
    raw_refs: Vec<String>,
    no_auto_commits: bool,
) -> Result<()> {
    let session = session_state::current_id();
    let mut linked = Vec::new();
    if !no_auto_commits && !raw_refs.iter().any(|item| item.starts_with("commit:")) {
        if let (Some(id), Some(common), Ok(cwd)) = (
            session.as_deref(),
            session_git::current_common_dir(),
            std::env::current_dir(),
        ) {
            let state = session_state::load(id)?;
            if state.repo_common.as_deref() == Some(common.as_str()) {
                linked = state
                    .commits
                    .iter()
                    .filter(|sha| !state.logged.contains(*sha) && session_git::reachable(&cwd, sha))
                    .cloned()
                    .collect();
            }
        }
    }
    let mut all_refs = raw_refs;
    all_refs.extend(linked.iter().map(|sha| format!("commit:{sha}")));
    let refs = refs::parse_all(&all_refs)?;
    let result = api.post(
        &format!("{}/logs", commands::path(tenant, project)?),
        &json!({"title": title, "body": body, "refs": refs}),
    )?;
    if let Some(id) = session.filter(|_| !linked.is_empty()) {
        let mut state = session_state::load(&id)?;
        state.logged.extend(linked);
        session_state::save(&id, &state)?;
    }
    output::json(&result)
}
