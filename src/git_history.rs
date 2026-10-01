use crate::git;

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
