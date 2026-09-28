use crate::api::Api;
use crate::{attribution, output};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;

fn git_at(path: &Path, args: &[&str]) -> Option<String> {
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

pub fn list(
    api: &Api,
    tenant: &str,
    name: Option<&str>,
    all_projects: bool,
    json_mode: bool,
) -> Result<()> {
    let query = if all_projects {
        String::new()
    } else {
        format!("?project={}", crate::project::slug(name)?)
    };
    let result = api.get(&format!("/api/v1/tenants/{tenant}/checkouts{query}"))?;
    let mut lines = Vec::new();
    for row in result["checkouts"].as_array().into_iter().flatten() {
        lines.push(format!(
            "  {:<24} {:<10} {:<24} {}",
            row["kind"].as_str().unwrap_or("main"),
            row["host"].as_str().unwrap_or(""),
            row["project_slug"].as_str().unwrap_or(""),
            row["path"].as_str().unwrap_or("")
        ));
    }
    output::emit(
        &result,
        &if lines.is_empty() {
            "  (no checkouts)".into()
        } else {
            lines.join("\n")
        },
        json_mode,
    )
}

pub fn current() -> Result<PathBuf> {
    checkout(&std::env::current_dir()?)
}

fn checkout(path: &Path) -> Result<PathBuf> {
    let root = git_at(path, &["rev-parse", "--show-toplevel"])
        .with_context(|| format!("{} is not inside a Git checkout", path.display()))?;
    Ok(PathBuf::from(root).canonicalize()?)
}

pub fn record(api: &Api, tenant: &str, slug: &str, path: &Path) -> Result<Value> {
    let root = checkout(path)?;
    let common = git_at(&root, &["rev-parse", "--git-common-dir"]).unwrap_or_default();
    let common = root.join(common).canonicalize().unwrap_or_default();
    let main = common
        .parent()
        .filter(|_| common.file_name().is_some_and(|name| name == ".git"));
    let worktree = main.is_some_and(|main| main != root);
    let result = api.post(
        &format!("/api/v1/tenants/{tenant}/projects/{slug}/paths"),
        &json!({
            "host": attribution::host(), "path": root,
            "kind": if worktree { "worktree" } else { "main" },
            "branch": git_at(&root, &["branch", "--show-current"]),
            "main_path": if worktree { main.map(Path::to_path_buf) } else { None },
        }),
    )?;
    Ok(result)
}

pub fn add(api: &Api, tenant: &str, slug: &str, path: &Path, json_mode: bool) -> Result<()> {
    let result = record(api, tenant, slug, path)?;
    output::emit(
        &result,
        &format!("Recorded checkout {} for {slug}.", path.display()),
        json_mode,
    )
}

pub fn remove(
    api: &Api,
    tenant: &str,
    slug: &str,
    path: &Path,
    host: String,
    json_mode: bool,
) -> Result<()> {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !path.is_absolute() {
        bail!("checkout path must be absolute");
    }
    let result = api.post(
        &format!("/api/v1/tenants/{tenant}/projects/{slug}/paths/remove"),
        &json!({"host": host, "path": path}),
    )?;
    output::emit(
        &result,
        &format!("Removed checkout {} from {slug}.", path.display()),
        json_mode,
    )
}
