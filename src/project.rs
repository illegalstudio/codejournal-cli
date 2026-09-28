use crate::git;
use anyhow::{Context, Result, bail};
use std::path::Path;

pub fn slug(explicit: Option<&str>) -> Result<String> {
    let value = match explicit {
        Some(value) => value.to_owned(),
        None => git::output(&["remote", "get-url", "origin"])
            .and_then(|remote| remote_slug(&remote))
            .unwrap_or_else(|| {
                git::root()
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
                    .file_name()
                    .and_then(|part| part.to_str())
                    .unwrap_or("project")
                    .to_owned()
            }),
    };
    let normalized = value.to_ascii_lowercase().replace(['_', ' '], "-");
    if normalized.is_empty()
        || !normalized
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        bail!("invalid project slug: {value}");
    }
    Ok(normalized)
}

fn remote_slug(remote: &str) -> Option<String> {
    let path = if remote.contains("://") {
        reqwest::Url::parse(remote).ok()?.path().to_owned()
    } else if let Some((_, path)) = remote.split_once(':') {
        path.to_owned()
    } else {
        remote.to_owned()
    };
    let parts: Vec<_> = path
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() < 2 {
        return None;
    }
    Some(parts[parts.len() - 2..].join("-"))
}

pub fn name(explicit: Option<&str>) -> Result<String> {
    match explicit {
        Some(value) => Ok(value.to_owned()),
        None => Ok(Path::new(&std::env::current_dir()?)
            .file_name()
            .and_then(|part| part.to_str())
            .context("cannot determine project name")?
            .to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::remote_slug;

    #[test]
    fn remote_identity_matches_imported_project_slugs() {
        for remote in [
            "git@github.com:illegalstudio/codejournal.git",
            "https://github.com/illegalstudio/codejournal.git",
            "ssh://git@github.com/illegalstudio/codejournal.git",
        ] {
            assert_eq!(remote_slug(remote).as_deref(), Some("illegalstudio-codejournal"));
        }
    }
}
