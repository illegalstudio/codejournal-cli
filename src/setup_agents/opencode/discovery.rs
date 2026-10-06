use super::managed;
use anyhow::{Context, Result};
use directories::BaseDirs;
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub fn home() -> Result<PathBuf> {
    Ok(BaseDirs::new()
        .context("home directory unavailable")?
        .home_dir()
        .to_path_buf())
}

pub fn default_root() -> Result<PathBuf> {
    Ok(std::env::var_os("XDG_CONFIG_HOME")
        .filter(|value| Path::new(value).is_absolute())
        .map(PathBuf::from)
        .unwrap_or(home()?.join(".config"))
        .join("opencode"))
}

pub fn root() -> Result<PathBuf> {
    Ok(std::path::absolute(
        std::env::var_os("OPENCODE_CONFIG_DIR")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or(default_root()?),
    )?)
}

pub fn same_path(left: &Path, right: &Path) -> bool {
    left == right
        || matches!((fs::canonicalize(left), fs::canonicalize(right)),
        (Ok(left), Ok(right)) if left == right)
}

fn enabled(name: &str) -> bool {
    !std::env::var(name).is_ok_and(|value| value == "1" || value == "true")
}

pub fn skills() -> Result<Vec<PathBuf>> {
    let mut roots = Vec::new();
    for root in [default_root()?, root()?, home()?.join(".opencode")] {
        roots.extend([root.join("skills"), root.join("skill")]);
    }
    if enabled("OPENCODE_DISABLE_EXTERNAL_SKILLS") {
        roots.push(home()?.join(".agents/skills"));
        if enabled("OPENCODE_DISABLE_CLAUDE_CODE") && enabled("OPENCODE_DISABLE_CLAUDE_CODE_SKILLS")
        {
            roots.push(home()?.join(".claude/skills"));
        }
    }
    let mut found = Vec::new();
    let mut visited = HashSet::new();
    let name = Regex::new(r#"(?m)^[ \t]*name:[ \t]*[\"']?code-journal[\"']?[ \t]*(?:#.*)?$"#)?;
    for root in roots {
        scan(&root, &name, &mut visited, &mut found)?;
    }
    found.sort();
    Ok(found)
}

fn scan(
    path: &Path,
    name: &Regex,
    visited: &mut HashSet<PathBuf>,
    found: &mut Vec<PathBuf>,
) -> Result<()> {
    if !path.is_dir() || !visited.insert(fs::canonicalize(path)?) {
        return Ok(());
    }
    let skill = path.join("SKILL.md");
    if let Some(body) = managed::read(&skill)? {
        let body = body.replace("\r\n", "\n");
        let frontmatter = body
            .trim_start_matches('\u{feff}')
            .strip_prefix("---\n")
            .and_then(|body| body.split_once("\n---").map(|(header, _)| header));
        if frontmatter.is_some_and(|header| name.is_match(header)) {
            found.push(skill);
        }
    }
    for item in fs::read_dir(path)? {
        scan(&item?.path(), name, visited, found)?;
    }
    Ok(())
}
