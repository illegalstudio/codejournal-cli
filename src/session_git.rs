use crate::git;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn common_dir(cwd: &Path) -> Option<String> {
    let raw = command(cwd, &["rev-parse", "--git-common-dir"])?;
    let directory = PathBuf::from(raw);
    let absolute = if directory.is_absolute() {
        directory
    } else {
        cwd.join(directory)
    };
    Some(absolute.canonicalize().ok()?.to_string_lossy().to_string())
}

pub fn commit_target(command_text: &str, cwd: &Path) -> Option<PathBuf> {
    let tokens = shell_words::split(command_text).ok()?;
    for (index, token) in tokens.iter().enumerate() {
        if token != "git" {
            continue;
        }
        let mut at = index + 1;
        let mut target = cwd.to_path_buf();
        if tokens.get(at).is_some_and(|part| part == "-C") {
            target = PathBuf::from(tokens.get(at + 1)?);
            if !target.is_absolute() {
                target = cwd.join(target);
            }
            at += 2;
        }
        if tokens.get(at).is_some_and(|part| part == "commit") {
            return Some(target);
        }
    }
    None
}

pub fn commit_from_output(cwd: &Path, output: &str) -> Option<String> {
    let pattern = regex::Regex::new(r"\[[^\]]+ ([0-9a-f]{7,40})\]").ok()?;
    let short = pattern.captures(output)?.get(1)?.as_str();
    let revision = format!("{short}^{{commit}}");
    command(cwd, &["rev-parse", "--verify", &revision])
}

pub fn reachable(cwd: &Path, sha: &str) -> bool {
    command(
        cwd,
        &[
            "for-each-ref",
            &format!("--contains={sha}"),
            "--format=%(refname)",
            "refs/heads",
            "refs/remotes",
        ],
    )
    .is_some_and(|refs| !refs.is_empty())
}

pub fn head(cwd: &Path) -> Option<String> {
    command(cwd, &["rev-parse", "HEAD"])
}

pub fn resolve_commit(cwd: &Path, sha: &str) -> Option<String> {
    if !(7..=40).contains(&sha.len()) || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    command(
        cwd,
        &["rev-parse", "--verify", &format!("{sha}^{{commit}}")],
    )
}

fn command(cwd: &Path, args: &[&str]) -> Option<String> {
    let result = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .ok()?;
    if !result.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&result.stdout).trim().to_owned())
}

pub fn current_common_dir() -> Option<String> {
    session_git_root().and_then(|root| common_dir(&root))
}

fn session_git_root() -> Option<PathBuf> {
    git::root()
}
