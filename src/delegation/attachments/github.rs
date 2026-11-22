//! Pull requests and issues read by the host's `gh`: the guest runs without the user's GitHub credentials.
use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;
use std::process::Command;

/// Room left for the description and discussion when a diff is cut to fit one attachment.
const DIFF_LIMIT: usize = 800_000;

/// `gh` arguments selecting the record: a URL as given, `owner/repo#N`, or `#N` in this repository.
fn target(reference: &str) -> Result<Vec<String>> {
    let reference = reference.trim();
    if reference.starts_with("https://") {
        return Ok(vec![reference.to_owned()]);
    }
    let (repository, number) = reference.rsplit_once('#').unwrap_or(("", reference));
    ensure!(
        !number.is_empty() && number.chars().all(|digit| digit.is_ascii_digit()),
        "invalid GitHub reference {reference}: pass a pull request or issue URL, owner/repo#N, or #N for this repository"
    );
    let mut args = vec![number.to_owned()];
    if !repository.is_empty() {
        args.extend(["--repo".to_owned(), repository.to_owned()]);
    }
    Ok(args)
}

fn gh(args: &[String]) -> Result<String> {
    let output = Command::new("gh")
        .args(args)
        .output()
        .context("--context-github needs the GitHub CLI (gh) on the host")?;
    if !output.status.success() {
        bail!(
            "{}",
            String::from_utf8_lossy(&output.stderr).trim().to_owned()
        );
    }
    String::from_utf8(output.stdout).context("gh returned text that is not UTF-8")
}

fn view(kind: &str, fields: &str, target: &[String]) -> Result<Value> {
    let mut args = vec![kind.to_owned(), "view".to_owned()];
    args.extend(target.iter().cloned());
    args.extend(["--json".to_owned(), fields.to_owned()]);
    Ok(serde_json::from_str(&gh(&args)?)?)
}

/// The record as Markdown, with its URL: description, discussion and, for a pull request, the diff.
pub fn read(reference: &str) -> Result<(String, String)> {
    let target = target(reference)?;
    let pull = view(
        "pr",
        "number,title,state,author,body,url,baseRefName,headRefName,comments,reviews",
        &target,
    );
    let (record, diff) = match pull {
        Ok(record) => {
            let mut args = vec!["pr".to_owned(), "diff".to_owned()];
            args.extend(target.iter().cloned());
            (record, Some(gh(&args)?))
        }
        Err(pull_error) => match view(
            "issue",
            "number,title,state,author,body,url,comments",
            &target,
        ) {
            Ok(record) => (record, None),
            Err(issue_error) => bail!(
                "the host's gh could not read {reference} as a pull request ({pull_error:#}) or as an issue ({issue_error:#})"
            ),
        },
    };
    let url = record["url"].as_str().unwrap_or(reference).to_owned();
    Ok((
        super::github_text::render(&record, diff.as_deref(), DIFF_LIMIT),
        url,
    ))
}

#[cfg(test)]
mod tests {
    use super::target;

    #[test]
    fn references_select_the_record_and_its_repository() {
        assert_eq!(target("#12").unwrap(), ["12"]);
        assert_eq!(target(" 12 ").unwrap(), ["12"]);
        assert_eq!(
            target("owner/repo#7").unwrap(),
            ["7", "--repo", "owner/repo"]
        );
        let url = "https://github.com/owner/repo/pull/7";
        assert_eq!(target(url).unwrap(), [url]);
        for invalid in ["owner/repo", "#", "--web", "#1 --web", "owner/repo#x"] {
            assert!(target(invalid).is_err(), "{invalid}");
        }
    }
}
