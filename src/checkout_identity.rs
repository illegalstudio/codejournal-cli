use crate::project;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Checkout {
    pub root: PathBuf,
    pub anchor: PathBuf,
    pub kind: &'static str,
    pub branch: Option<String>,
    pub origin: Option<String>,
}

impl Checkout {
    pub fn main_path(&self) -> Option<&Path> {
        (self.anchor != self.root).then_some(self.anchor.as_path())
    }
}

pub fn current() -> Option<Checkout> {
    at(&std::env::current_dir().ok()?, 0)
}

pub fn at(path: &Path, depth: usize) -> Option<Checkout> {
    let root = PathBuf::from(git(path, &["rev-parse", "--show-toplevel"])?)
        .canonicalize()
        .ok()?;
    let branch = git(&root, &["symbolic-ref", "--quiet", "--short", "HEAD"]);
    let origin =
        git(&root, &["remote", "get-url", "origin"]).map(|raw| project::normalize_remote(&raw));
    let mut checkout = Checkout {
        anchor: root.clone(),
        root,
        kind: "main",
        branch,
        origin,
    };
    let git_dir = git(&checkout.root, &["rev-parse", "--absolute-git-dir"])
        .and_then(|raw| PathBuf::from(raw).canonicalize().ok());
    let common = git(
        &checkout.root,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
    .and_then(|raw| PathBuf::from(raw).canonicalize().ok());
    if let (Some(git_dir), Some(common)) = (git_dir, common) {
        if git_dir != common {
            checkout.kind = "worktree";
            checkout.anchor = if common.file_name().is_some_and(|name| name == ".git") {
                common.parent()?.to_path_buf()
            } else {
                common
            };
            return Some(checkout);
        }
    }
    let main_url = git(&checkout.root, &["config", "--get", "remote.main.url"]);
    let source = main_url
        .as_deref()
        .and_then(|url| local_remote(url, &checkout.root));
    if let Some(source) = source.filter(|_| depth < 8) {
        if let Some(source_info) = source.is_dir().then(|| at(&source, depth + 1)).flatten() {
            let foreign = checkout
                .origin
                .as_ref()
                .zip(source_info.origin.as_ref())
                .is_some_and(|(left, right)| left != right);
            if !foreign {
                checkout.kind = "cow";
                checkout.anchor = source_info.anchor;
            }
        } else {
            checkout.kind = "cow";
        }
    }
    Some(checkout)
}

fn local_remote(url: &str, base: &Path) -> Option<PathBuf> {
    let raw = url.trim().strip_prefix("file://").unwrap_or(url.trim());
    if raw.contains("://") || (!url.starts_with("file://") && raw.contains(':')) {
        return None;
    }
    let path = Path::new(raw);
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    };
    Some(path.canonicalize().unwrap_or(path))
}

fn git(path: &Path, args: &[&str]) -> Option<String> {
    let result = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .ok()?;
    result
        .status
        .success()
        .then(|| String::from_utf8_lossy(&result.stdout).trim().to_owned())
}
