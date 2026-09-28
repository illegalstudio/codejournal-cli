use crate::api::Api;
use crate::{attribution, checkout_identity, output};
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
        format!(
            "?project={}",
            crate::project_bootstrap::resolved_slug(api, tenant, name)?
        )
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
    let checkout = checkout_identity::at(path, 0)
        .with_context(|| format!("{} is not inside a Git checkout", path.display()))?;
    let result = api.post(
        &format!("/api/v1/tenants/{tenant}/projects/{slug}/paths"),
        &json!({
            "host": attribution::host(), "path": checkout.root,
            "kind": checkout.kind, "branch": checkout.branch,
            "main_path": checkout.main_path(),
        }),
    )?;
    Ok(result)
}

pub fn add(api: &Api, tenant: &str, slug: &str, path: &Path, json_mode: bool) -> Result<()> {
    let checkout = checkout_identity::at(path, 0)
        .with_context(|| format!("{} is not inside a Git checkout", path.display()))?;
    let project = api.get(&format!("/api/v1/tenants/{tenant}/projects/{slug}"))?;
    if let (Some(remote), Some(stored)) = (
        checkout.origin.as_deref(),
        project["project"]["remote_url"].as_str(),
    ) {
        if remote != stored {
            bail!(
                "{} has remote {remote}, but {slug} uses {stored}; update the project remote first with `cj project edit --remote` or `--from-git`",
                path.display()
            );
        }
    }
    let result = record(api, tenant, slug, path)?;
    output::emit(
        &result,
        &format!(
            "Recorded {} checkout {} for {slug}.",
            checkout.kind,
            checkout.root.display()
        ),
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
