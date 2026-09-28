use crate::git;
use anyhow::{Context, Result};
use regex::Regex;

pub fn expand(raw: &str) -> Option<Result<String>> {
    let pattern = Regex::new(
        r"(?i)^((?<repo>[\w.-]+/[\w.-]+))?(?<kind>pr|pull|mr|issue)?(?<sep>[#!])(?<number>\d+)$",
    )
    .ok()?;
    pattern.captures(raw)?;
    expand_with_origin(raw, git::forge_origin())
}

fn expand_with_origin(raw: &str, origin: Option<(String, String)>) -> Option<Result<String>> {
    let pattern = Regex::new(
        r"(?i)^((?<repo>[\w.-]+/[\w.-]+))?(?<kind>pr|pull|mr|issue)?(?<sep>[#!])(?<number>\d+)$",
    )
    .ok()?;
    let found = pattern.captures(raw)?;
    let number = &found["number"];
    let change = &found["sep"] == "!"
        || found.name("kind").is_some_and(|part| {
            matches!(
                part.as_str().to_ascii_lowercase().as_str(),
                "pr" | "pull" | "mr"
            )
        });
    if !change && found.name("repo").is_none() {
        return Some(Ok(format!("issue:#{number}")));
    }
    let target = match found.name("repo") {
        Some(repo) => Ok((
            origin
                .as_ref()
                .map(|item| item.0.as_str())
                .unwrap_or("github.com")
                .to_owned(),
            repo.as_str().to_owned(),
        )),
        None => origin.context(format!("{raw} needs a GitHub or GitLab origin remote")),
    };
    Some(target.map(|(host, project)| {
        let segment = if host == "gitlab.com" {
            if change {
                "-/merge_requests"
            } else {
                "-/issues"
            }
        } else if change {
            "pull"
        } else {
            "issues"
        };
        format!("url:https://{host}/{project}/{segment}/{number}")
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shorthand_uses_origin_and_rejects_missing_change_request_origin() {
        let origin = Some(("github.com".to_owned(), "nahime/demo".to_owned()));
        assert_eq!(
            expand_with_origin("PR#42", origin.clone())
                .unwrap()
                .unwrap(),
            "url:https://github.com/nahime/demo/pull/42"
        );
        assert_eq!(
            expand_with_origin("getelephc/laravel#3", origin)
                .unwrap()
                .unwrap(),
            "url:https://github.com/getelephc/laravel/issues/3"
        );
        assert!(expand_with_origin("MR!4", None).unwrap().is_err());
    }
}
