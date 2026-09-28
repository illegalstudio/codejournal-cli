use crate::checkout_identity;
use anyhow::{Context, Result, bail};

pub fn slug(explicit: Option<&str>) -> Result<String> {
    let value = match explicit {
        Some(value) => value.to_owned(),
        None => checkout_identity::current()
            .map(|checkout| slug_for(&checkout))
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .unwrap_or_default()
                    .file_name()
                    .and_then(|part| part.to_str())
                    .map(slug_from_identity)
                    .unwrap_or_else(|| "project".to_owned())
            }),
    };
    if value.is_empty()
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
    {
        bail!("invalid project slug: {value}");
    }
    Ok(value)
}

pub fn slug_for(checkout: &checkout_identity::Checkout) -> String {
    checkout
        .origin
        .as_deref()
        .map(slug_from_identity)
        .or_else(|| {
            checkout
                .anchor
                .file_name()
                .and_then(|part| part.to_str())
                .map(slug_from_identity)
        })
        .unwrap_or_else(|| "project".to_owned())
}

fn slug_from_identity(identity: &str) -> String {
    let parts: Vec<_> = identity
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    let start = if parts.len() >= 3 {
        parts.len() - 2
    } else {
        parts.len().saturating_sub(1)
    };
    let candidate = parts[start..].join("-").to_ascii_lowercase();
    let mut sanitized = String::new();
    let mut replacing = false;
    for character in candidate.chars() {
        if character.is_ascii_lowercase() || character.is_ascii_digit() || "._-".contains(character)
        {
            sanitized.push(character);
            replacing = false;
        } else if !replacing {
            sanitized.push('-');
            replacing = true;
        }
    }
    let trimmed = sanitized.trim_matches(['-', '.']);
    if trimmed.is_empty() {
        "project".to_owned()
    } else {
        trimmed.to_owned()
    }
}

pub fn normalize_remote(raw: &str) -> String {
    let value = raw.trim();
    let host_path = if let Ok(url) = reqwest::Url::parse(value) {
        let authority = &value[value.find("://").map(|index| index + 3).unwrap_or(0)..];
        let authority = authority.split('/').next().unwrap_or("");
        let host = authority.rsplit('@').next().unwrap_or(authority);
        format!("{}/{}", host, url.path().trim_start_matches('/'))
    } else if let Some((left, right)) = value.split_once(':') {
        format!("{}/{}", left.trim_start_matches("git@"), right)
    } else {
        value.trim_start_matches("git@").to_owned()
    };
    let normalized = host_path.trim_end_matches('/').to_ascii_lowercase();
    normalized
        .strip_suffix(".git")
        .unwrap_or(&normalized)
        .to_owned()
}

pub fn name(explicit: Option<&str>) -> Result<String> {
    match explicit {
        Some(value) => Ok(value.to_owned()),
        None => {
            if let Some(checkout) = checkout_identity::current() {
                return Ok(name_for(&checkout));
            }
            Ok(std::env::current_dir()?
                .file_name()
                .and_then(|part| part.to_str())
                .context("cannot determine project name")?
                .to_owned())
        }
    }
}

pub fn name_for(checkout: &checkout_identity::Checkout) -> String {
    if let Some(remote) = &checkout.origin {
        remote.rsplit('/').next().unwrap_or(remote).to_owned()
    } else {
        checkout
            .anchor
            .file_name()
            .and_then(|part| part.to_str())
            .unwrap_or("project")
            .to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_remote, slug_from_identity};

    #[test]
    fn remote_identity_matches_imported_project_slugs() {
        for remote in [
            "git@github.com:illegalstudio/codejournal.git",
            "https://github.com/illegalstudio/codejournal.git",
            "ssh://git@github.com/illegalstudio/codejournal.git",
        ] {
            assert_eq!(
                slug_from_identity(&normalize_remote(remote)),
                "illegalstudio-codejournal"
            );
        }
    }

    #[test]
    fn remote_identity_matches_python_normalization() {
        for (remote, expected) in [
            (
                "git@github.com:illegalstudio/codejournal.git",
                "github.com/illegalstudio/codejournal",
            ),
            (
                "https://github.com/illegalstudio/codejournal.git",
                "github.com/illegalstudio/codejournal",
            ),
            (
                "ssh://git@git.home.arpa:2222/nahime/ai.git",
                "git.home.arpa:2222/nahime/ai",
            ),
            (
                "https://user:pw@gitlab.com/group/sub/repo/",
                "gitlab.com/group/sub/repo",
            ),
            ("https://github.com/Team/App.GIT", "github.com/team/app"),
        ] {
            assert_eq!(normalize_remote(remote), expected);
        }
    }

    #[test]
    fn identity_slugs_match_python_for_short_and_dotted_paths() {
        assert_eq!(slug_from_identity("example.com/repo"), "repo");
        assert_eq!(slug_from_identity("gitlab.com/group/sub/repo"), "sub-repo");
        assert_eq!(
            slug_from_identity("github.com/team/my_repo"),
            "team-my_repo"
        );
        assert_eq!(slug_from_identity("my project"), "my-project");
        assert_eq!(slug_from_identity("my   project"), "my-project");
    }
}
