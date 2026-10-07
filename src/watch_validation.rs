use crate::{cli::Command, watch_args::WatchAction};
use anyhow::{Result, bail};

pub fn timeout(value: &str) -> Result<u64, String> {
    let (digits, factor) = [('s', 1), ('m', 60), ('h', 3600), ('d', 86400)]
        .into_iter()
        .find_map(|(suffix, factor)| value.strip_suffix(suffix).map(|digits| (digits, factor)))
        .unwrap_or((value, 1));
    let seconds = (!digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| {
            digits
                .parse::<u64>()
                .ok()
                .and_then(|n| n.checked_mul(factor))
        })
        .flatten();
    seconds.filter(|n| (1..=86400).contains(n)).ok_or_else(|| {
        "timeout must be 1-86400 seconds, or a duration such as 30s, 10m, 3h or 1d".to_owned()
    })
}

pub fn preflight(command: &Command) -> Result<()> {
    if let Command::Watch {
        action: WatchAction::Cancel { id },
    } = command
    {
        let normalized = id.replace('-', "");
        if id.len() > 36
            || !(8..=32).contains(&normalized.len())
            || !normalized.chars().all(|value| value.is_ascii_hexdigit())
        {
            bail!(
                "watch ID must be a UUID or a prefix of at least 8 hexadecimal characters; nothing was sent"
            );
        }
        return Ok(());
    }

    let Command::Watch {
        action: WatchAction::Start { command, .. },
    } = command
    else {
        return Ok(());
    };
    if command.len() > 100 {
        bail!(
            "watch command accepts at most 100 arguments including the executable; nothing was sent or started"
        );
    }
    for (index, value) in command.iter().enumerate() {
        if value.trim().is_empty() || value.chars().count() > 2000 {
            bail!(
                "watch command item {} must contain 1-2000 characters; use a script file for longer code. Nothing was sent or started",
                index + 1
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{preflight, timeout};
    use crate::{cli::Cli, watch_args::WatchAction};
    use clap::Parser;

    #[test]
    fn command_budgets_are_validated_without_authentication_or_execution() {
        let parsed =
            Cli::try_parse_from(["cj", "watch", "start", "--title", "Test", "--", "true"]).unwrap();
        let crate::cli::Command::Watch {
            action: WatchAction::Start { command, .. },
        } = parsed.command
        else {
            panic!("expected watch start");
        };
        assert_eq!(command, ["true"]);
        for (argument, accepted) in [
            ("è".repeat(2000), true),
            ("è".repeat(2001), false),
            (String::new(), false),
        ] {
            let cli = Cli::try_parse_from([
                "cj", "watch", "start", "--title", "Test", "--", "true", &argument,
            ])
            .unwrap();
            assert_eq!(preflight(&cli.command).is_ok(), accepted);
        }
        for (count, accepted) in [(100, true), (101, false)] {
            let mut args = vec!["cj", "watch", "start", "--title", "Test", "--"];
            args.extend(std::iter::repeat_n("true", count));
            let cli = Cli::try_parse_from(args).unwrap();
            assert_eq!(preflight(&cli.command).is_ok(), accepted);
        }
    }

    #[test]
    fn durations_match_the_server_range_without_overflow() {
        for (value, expected) in [
            ("1", 1),
            ("30s", 30),
            ("10m", 600),
            ("3h", 10800),
            ("24h", 86400),
            ("1d", 86400),
        ] {
            assert_eq!(timeout(value).unwrap(), expected);
        }
        for value in [
            "0",
            "0s",
            "25h",
            "86401",
            "-1",
            "1.5h",
            "+1",
            "h",
            "",
            "18446744073709551615h",
        ] {
            assert!(timeout(value).is_err());
        }
    }
}
