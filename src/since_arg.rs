pub const HELP: &str =
    "Since today, yesterday, a duration such as 30d or 12h, or a UTC date YYYY-MM-DD";

pub fn parse(value: &str) -> Result<String, String> {
    let relative = value.strip_suffix('d').or_else(|| value.strip_suffix('h'));
    let valid = matches!(value, "today" | "yesterday")
        || relative.is_some_and(|digits| {
            !digits.starts_with('0')
                && digits.len() <= 4
                && digits.bytes().all(|b| b.is_ascii_digit())
                && !digits.is_empty()
        })
        || (value.len() == 10 && chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok());
    if valid {
        Ok(value.to_owned())
    } else {
        Err(HELP.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn dates_and_durations_match_the_api_contract() {
        for value in ["today", "yesterday", "30d", "12h", "2026-09-30"] {
            assert!(parse(value).is_ok());
        }
        for value in [
            "30 days ago",
            "0d",
            "01h",
            "10000d",
            "-1h",
            "2026-02-31",
            "2026-9-30",
        ] {
            assert!(parse(value).is_err());
        }
    }
}
