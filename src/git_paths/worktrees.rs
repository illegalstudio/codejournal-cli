use crate::git;
use std::path::{Path, PathBuf};

pub fn active(root: &Path) -> Vec<(String, PathBuf)> {
    git::output(&["worktree", "list", "--porcelain", "-z"])
        .unwrap_or_default()
        .split("\0\0")
        .filter_map(|record| {
            let fields = record.split('\0').collect::<Vec<_>>();
            let path = fields
                .iter()
                .find_map(|field| field.strip_prefix("worktree "))?;
            let path = PathBuf::from(path);
            if path.as_os_str().is_empty() || path.canonicalize().ok() == root.canonicalize().ok() {
                return None;
            }
            let label = if let Some(branch) = fields
                .iter()
                .find_map(|field| field.strip_prefix("branch refs/heads/"))
            {
                if !super::safe_branch(branch) {
                    return None;
                }
                branch.to_owned()
            } else {
                let head = fields
                    .iter()
                    .find_map(|field| field.strip_prefix("HEAD "))?;
                if !super::safe_commit(head) {
                    return None;
                }
                format!("detached {}", head.chars().take(12).collect::<String>())
            };
            Some((label, path))
        })
        .take(30)
        .collect()
}

pub fn containing(worktrees: &[(String, PathBuf)], path: &str) -> Vec<String> {
    worktrees
        .iter()
        .filter(|(_, root)| root.join(path).symlink_metadata().is_ok())
        .map(|(branch, _)| branch.clone())
        .collect()
}
