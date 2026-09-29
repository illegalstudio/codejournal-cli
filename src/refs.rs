use crate::{path_ref, shorthand};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

/// Help text shared by every `--ref` flag.
pub const HELP: &str = "Reference, repeatable: path:FILE, commit:SHA, branch:NAME, issue:#N, a URL, or GitHub shorthand such as owner/repo#12";

pub fn parse_all(values: &[String]) -> Result<Vec<Value>> {
    let mut parsed = Vec::new();
    for value in values {
        let item = parse(value)?;
        if !parsed.contains(&item) {
            parsed.push(item);
        }
    }
    Ok(parsed)
}

fn parse(raw: &str) -> Result<Value> {
    let raw = raw.trim();
    let expanded = if raw.starts_with("https://") || raw.starts_with("http://") {
        format!("url:{raw}")
    } else if let Some(value) = shorthand::expand(raw) {
        value?
    } else {
        raw.to_owned()
    };
    let (kind, value) = expanded
        .split_once(':')
        .context("ref must be KIND:VALUE, a URL, or GitHub shorthand")?;
    let kind = kind.trim().to_ascii_lowercase();
    let value = value.trim();
    if !["path", "commit", "branch", "url", "issue"].contains(&kind.as_str()) || value.is_empty() {
        bail!("invalid ref: {raw}");
    }
    let value = if kind == "path" {
        path_ref::normalize(value)?
    } else {
        value.to_owned()
    };
    Ok(json!({"kind": kind, "value": value}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issues_and_urls_are_normalized_without_a_remote() {
        assert_eq!(
            parse("#42").unwrap(),
            json!({"kind": "issue", "value": "#42"})
        );
        assert_eq!(
            parse("issue#5").unwrap(),
            json!({"kind": "issue", "value": "#5"})
        );
        assert_eq!(
            parse("path:/docs/").unwrap(),
            json!({"kind": "path", "value": "docs"})
        );
        assert!(parse("path:../secret").is_err());
    }
}
