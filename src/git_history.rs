use crate::git;
use serde_json::{Value, json};

pub fn touching_commits(created_at: Option<&str>, paths: &[&str]) -> Vec<Value> {
    let Some(since) = created_at.and_then(parse_date) else {
        return Vec::new();
    };
    if paths.is_empty() {
        return Vec::new();
    }
    let cutoff = format!(
        "--since={}",
        (since + chrono::Duration::seconds(1)).format("%Y-%m-%dT%H:%M:%SZ")
    );
    let mut args = vec![
        "log",
        "-10",
        "--format=CJ-COMMIT:%H",
        "--name-only",
        &cutoff,
        "HEAD",
        "--",
    ];
    args.extend(paths);
    let Some(output) = git::output(&args) else {
        return Vec::new();
    };
    let mut commits: Vec<Value> = Vec::new();
    for line in output.lines().filter(|line| !line.is_empty()) {
        if let Some(sha) = line.strip_prefix("CJ-COMMIT:").filter(|sha| {
            (40..=64).contains(&sha.len()) && sha.bytes().all(|byte| byte.is_ascii_hexdigit())
        }) {
            commits.push(json!({"sha":sha,"paths":[],"paths_omitted":0}));
        } else if let Some(commit) = commits.last_mut() {
            if line.chars().count() <= 1000
                && commit["paths"]
                    .as_array()
                    .is_some_and(|paths| paths.len() < 20)
            {
                if let Some(paths) = commit["paths"].as_array_mut() {
                    paths.push(json!(line));
                }
            } else {
                commit["paths_omitted"] =
                    json!(commit["paths_omitted"].as_u64().unwrap_or_default() + 1);
            }
        }
    }
    commits
}

pub fn changes_since(created_at: Option<&str>, paths: &[&str]) -> u64 {
    if paths.is_empty() {
        return 0;
    }
    let Some(since) = created_at.and_then(parse_date) else {
        return 0;
    };
    let after = since + chrono::Duration::seconds(1);
    let cutoff = format!("--since={}", after.format("%Y-%m-%dT%H:%M:%SZ"));
    let mut args = vec!["rev-list", "--count", cutoff.as_str(), "HEAD", "--"];
    args.extend(paths);
    git::output(&args)
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

fn parse_date(value: &str) -> Option<chrono::DateTime<chrono::FixedOffset>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .or_else(|| chrono::DateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f%#z").ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_api_timestamps_from_json_and_postgres() {
        let expected = parse_date("2026-10-01T08:00:00Z").unwrap();
        assert_eq!(parse_date("2026-10-01 08:00:00+00").unwrap(), expected);
        assert_eq!(
            parse_date("2026-10-01 08:00:00.123456+00")
                .unwrap()
                .timestamp(),
            expected.timestamp()
        );
    }
}
