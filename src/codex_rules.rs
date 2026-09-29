use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

const MARKER: &str = "# Managed by cj setup agents";

// Codex runs rule-allowed commands outside its sandbox, where cj can reach the
// keyring and the API. Commands with local side effects stay sandboxed.
const RULES: &str = r#"# Managed by cj setup agents; local changes are overwritten.
prefix_rule(
    pattern = ["cj", ["brief", "search", "recent", "show", "topics", "add", "supersede",
                      "obsolete", "answer", "entry", "log", "task", "plan", "doc", "rules",
                      "feedback", "notify", "garden", "digest", "status", "whoami", "sync",
                      "project", "projects", "checkouts", "refs", "activity", "notifications"]],
    decision = "allow",
    justification = "Code Journal reads its token from the system keyring and calls the hosted API",
    match = ["cj brief", "cj add --kind gotcha --title x"],
    not_match = ["cj watch start -- ls", "cj login"],
)
"#;

pub fn path(codex_home: &Path) -> PathBuf {
    codex_home.join("rules/code-journal.rules")
}

pub fn installed(path: &Path) -> bool {
    fs::read_to_string(path).is_ok_and(|content| content == RULES)
}

/// Installs or removes the managed rules file and reports whether it changed.
pub fn apply(path: &Path, install: bool, dry_run: bool) -> Result<bool> {
    let current = fs::read_to_string(path).ok();
    let changed = if install {
        current.as_deref() != Some(RULES)
    } else {
        current.is_some_and(|content| content.starts_with(MARKER))
    };
    if !changed || dry_run {
        return Ok(changed);
    }
    if !install {
        fs::remove_file(path)?;
        return Ok(true);
    }
    let parent = path.parent().context("invalid Codex rules path")?;
    fs::create_dir_all(parent)?;
    let temporary = path.with_extension(format!("cj-tmp-{}", uuid::Uuid::new_v4()));
    fs::write(&temporary, RULES)?;
    fs::rename(temporary, path)?;
    Ok(true)
}
