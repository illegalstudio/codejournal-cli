use crate::{session_git, session_state};
use anyhow::Result;

pub fn pending(explicit_refs: &[String], no_auto: bool) -> Result<Vec<String>> {
    if no_auto || explicit_refs.iter().any(|item| item.starts_with("commit:")) {
        return Ok(Vec::new());
    }
    let (Some(id), Some(common), Ok(cwd)) = (
        session_state::current_id(),
        session_git::current_common_dir(),
        std::env::current_dir(),
    ) else {
        return Ok(Vec::new());
    };
    let state = session_state::load(&id)?;
    if state.repo_common.as_deref() != Some(common.as_str()) {
        return Ok(Vec::new());
    }
    Ok(state
        .commits
        .iter()
        .filter(|sha| !state.logged.contains(*sha) && session_git::reachable(&cwd, sha))
        .cloned()
        .collect())
}

pub fn record(refs: &[String]) -> Result<()> {
    let Some(id) = session_state::current_id() else {
        return Ok(());
    };
    let mut state = session_state::load(&id)?;
    if let (Some(common), Ok(cwd)) = (session_git::current_common_dir(), std::env::current_dir()) {
        if state.repo_common.as_deref() == Some(common.as_str()) {
            for value in refs.iter().filter_map(|raw| raw.strip_prefix("commit:")) {
                if let Some(sha) = session_git::resolve_commit(&cwd, value) {
                    if !state.logged.contains(&sha) {
                        state.logged.push(sha);
                    }
                }
            }
        }
    }
    state.last_log_at = Some(session_state::now());
    session_state::save(&id, &state)
}
