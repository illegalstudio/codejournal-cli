use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

const MARKER: &str = "# Managed by cj setup agents";

// Keep automatic approval limited to reads and explicit inline bodies.
// Local configuration and arbitrary file uploads require ordinary command review.
const RULES: &str = r#"# Managed by cj setup agents; local changes are overwritten.
prefix_rule(
    pattern = ["cj", ["brief", "search", "recent", "show", "topics", "digest", "status",
                      "whoami", "projects", "checkouts", "activity"]],
    decision = "allow",
    justification = "Read journal data using the configured service",
    match = ["cj brief"],
    not_match = ["cj notify --body-file private.txt", "cj notifications config --ntfy https://example.test"],
)
prefix_rule(
    pattern = ["cj", ["log", "task", "plan", "doc", "feedback", "project", "refs", "notifications"], ["list", "show"]],
    decision = "allow",
    justification = "Read journal records without local configuration changes",
)
prefix_rule(pattern = ["cj", "rules", "show"], decision = "allow")
prefix_rule(
    pattern = ["cj", ["add", "notify"], "--body"],
    decision = "allow",
    justification = "Save an explicit inline body; the CLI rejects simultaneous body-file input",
    match = ["cj add --body fact --kind gotcha --title x"],
    not_match = ["cj add --body-file private.txt", "cj watch start -- ls", "cj login"],
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
