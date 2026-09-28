use crate::checkout_identity;
use anyhow::{Context, Result, bail};

pub fn slug(explicit: Option<&str>) -> Result<String> {
    let value = match explicit {
        Some(value) => value.to_owned(),
        None => checkout_identity::current()
            .and_then(|checkout| {
                checkout
                    .origin
                    .as_deref()
                    .and_then(remote_slug)
                    .or_else(|| {
                        checkout
                            .anchor
                            .file_name()
                            .and_then(|part| part.to_str())
                            .map(str::to_owned)
                    })
            })
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .unwrap_or_default()
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

pub fn normalize_remote(raw: &str) -> String {
    let value = raw.trim();
    let host_path = if let Ok(url) = reqwest::Url::parse(value) {
        format!(
            "{}/{}",
            url.host_str().unwrap_or(""),
            url.path().trim_start_matches('/')
        )
    } else if let Some((left, right)) = value.split_once(':') {
        format!("{}/{}", left.trim_start_matches("git@"), right)
    } else {
        value.trim_start_matches("git@").to_owned()
    };
    host_path
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .to_ascii_lowercase()
}

pub fn name(explicit: Option<&str>) -> Result<String> {
    match explicit {
        Some(value) => Ok(value.to_owned()),
        None => {
            let checkout = checkout_identity::current();
            if let Some(remote) = checkout
                .as_ref()
                .and_then(|checkout| checkout.origin.as_ref())
            {
                return Ok(remote.rsplit('/').next().unwrap_or(remote).to_owned());
            }
            Ok(checkout
                .map(|checkout| checkout.anchor)
                .unwrap_or(std::env::current_dir()?)
                .file_name()
                .and_then(|part| part.to_str())
                .context("cannot determine project name")?
                .to_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_remote, remote_slug};

    #[test]
    fn remote_identity_matches_imported_project_slugs() {
        for remote in [
            "git@github.com:illegalstudio/codejournal.git",
            "https://github.com/illegalstudio/codejournal.git",
            "ssh://git@github.com/illegalstudio/codejournal.git",
        ] {
            assert_eq!(
                remote_slug(remote).as_deref(),
                Some("illegalstudio-codejournal")
            );
        }
    }

    #[test]
    fn remote_identity_matches_python_normalization() {
        for remote in [
            "git@github.com:illegalstudio/codejournal.git",
            "https://github.com/illegalstudio/codejournal.git",
        ] {
            assert_eq!(
                normalize_remote(remote),
                "github.com/illegalstudio/codejournal"
            );
        }
    }
}
