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
    let checkout = session_git::checkout(&cwd);
    let own_checkout =
        |sha: &str| state.commit_checkouts.get(sha).or(state.root.as_ref()) == checkout.as_ref();
    Ok(state
        .commits
        .iter()
        .filter(|sha| {
            !state.logged.contains(*sha) && own_checkout(sha) && session_git::reachable(&cwd, sha)
        })
        .cloned()
        .collect())
}

pub fn notice(explicit_refs: &[String], no_auto: bool) -> Result<Option<String>> {
    if no_auto || explicit_refs.iter().any(|item| item.starts_with("commit:")) {
        return Ok(None);
    }
    let (Some(id), Some(common), Ok(cwd)) = (
        session_state::current_id(),
        session_git::current_common_dir(),
        std::env::current_dir(),
    ) else {
        return Ok(None);
    };
    let state = session_state::load(&id)?;
    if state.repo_common.as_deref() != Some(common.as_str()) {
        return Ok(None);
    }
    if let Some(head) = session_git::head(&cwd)
        && state
            .turn_head
            .as_ref()
            .is_some_and(|previous| previous != &head)
        && !state.commits.contains(&head)
        && !state.logged.contains(&head)
        && !session_state::others()?
            .iter()
            .any(|other| other.repo_common == state.repo_common && other.commits.contains(&head))
    {
        return Ok(Some(format!(
            "HEAD changed outside captured session commits and is now {}. No commits were inferred from that change. If it belongs to this session, pass --ref commit:HEAD (or an explicit SHA) to cj log add, or add it to an existing log with cj log update LOG_ID --add-ref commit:SHA.",
            label(&cwd, &head)
        )));
    }
    Ok(None)
}

/// "1a2b3c4d Subject", so a linked or missing commit can be recognized without looking it up.
fn label(cwd: &std::path::Path, sha: &str) -> String {
    let short = &sha[..sha.len().min(8)];
    match session_git::subject(cwd, sha) {
        Some(subject) => format!("{short} {subject}"),
        None => short.to_owned(),
    }
}

/// Names the automatically linked commits and the checkout they were captured in.
pub fn describe(linked: &[String], warned: bool) -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    if linked.is_empty() {
        return None;
    }
    let mut text = format!(
        "Linked {} commit(s) this session captured in {} since your previous log:",
        linked.len(),
        session_git::checkout(&cwd).unwrap_or_else(|| cwd.to_string_lossy().into_owned())
    );
    for sha in linked {
        text.push_str(&format!("\n  {}", label(&cwd, sha)));
    }
    if warned {
        text.push_str("\nThese are earlier captures, not the new HEAD named in the warning. If this log is not about them, replace its references with cj log update LOG_ID --ref commit:SHA.");
    }
    Some(text)
}

pub fn record(refs: &[String], log_id: Option<&str>) -> Result<()> {
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
    if let Some(id) = log_id.filter(|id| !id.is_empty()) {
        if !state.log_ids.iter().any(|saved| saved == id) {
            state.log_ids.push(id.to_owned());
        }
    }
    session_state::save(&id, &state)
}

pub fn amendment(log_id: &str) -> Result<()> {
    let Some(id) = session_state::current_id() else {
        return Ok(());
    };
    let mut state = session_state::load(&id)?;
    let prefix = log_id.replace('-', "");
    if prefix.len() >= 8
        && state.repo_common == session_git::current_common_dir()
        && state
            .log_ids
            .iter()
            .filter(|saved| saved.replace('-', "").starts_with(&prefix))
            .count()
            == 1
    {
        state.last_log_at = Some(session_state::now());
        session_state::save(&id, &state)?;
    }
    Ok(())
}
