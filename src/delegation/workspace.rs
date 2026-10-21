use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::process::Command;

pub fn ensure_writer_available(db: &rusqlite::Connection) -> Result<()> {
    let mut statement = db.prepare("SELECT * FROM delegations WHERE mode='write'")?;
    let sessions = statement
        .query_map([], super::store::row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ensure!(
        sessions.iter().all(|session| {
            !super::process::owns_processes(session)
                && super::terminal(session)
                && (super::text(session, "status") != "completed"
                    || !session["integrated_at"].is_null())
        }),
        "another write delegation is still unresolved"
    );
    Ok(())
}

pub fn git(cwd: &str, args: &[&str]) -> Result<String> {
    let output = Command::new("git").arg("-C").arg(cwd).args(args).output()?;
    ensure!(
        output.status.success(),
        "Git operation failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

pub fn inspect(cwd: &str) -> Result<(String, String)> {
    let root = git(cwd, &["rev-parse", "--show-toplevel"])
        .context("write delegation requires a Git worktree with an existing commit")?;
    let head = git(&root, &["rev-parse", "--verify", "HEAD"])?;
    ensure!(
        status(&root)?.is_empty(),
        "write delegation requires a clean Git worktree"
    );
    Ok((root, head))
}

fn status(root: &str) -> Result<String> {
    git(root, &["status", "--porcelain=v1", "--untracked-files=all"])
}

pub fn changed(session: &Value, verify_head: bool) -> Result<String> {
    let root = super::text(session, "workspace_root");
    if root.is_empty() {
        return Ok(String::new());
    }
    let head = git(root, &["rev-parse", "--verify", "HEAD"])?;
    ensure!(
        !verify_head || head == super::text(session, "baseline_sha"),
        "write guest changed HEAD; the host must create commits after review"
    );
    status(root)
}
