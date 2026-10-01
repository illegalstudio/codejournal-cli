use crate::cli::Cli;
use clap::builder::PossibleValuesParser;
use clap::{Command, CommandFactory, FromArgMatches};

pub fn parse() -> Cli {
    let matches = command().get_matches();
    Cli::from_arg_matches(&matches).unwrap_or_else(|error| error.exit())
}

fn command() -> Command {
    Cli::command()
        .mut_subcommand("plan", |cmd| {
            statuses(cmd, &["draft", "active", "done", "abandoned"])
        })
        .mut_subcommand("doc", |cmd| {
            statuses(cmd, &["draft", "current", "outdated"])
        })
}

fn statuses(command: Command, values: &'static [&'static str]) -> Command {
    let list = [values, &["open", "all"]].concat();
    command
        .mut_subcommand("list", |cmd| {
            cmd.mut_arg("status", |arg| {
                arg.value_parser(PossibleValuesParser::new(list))
            })
        })
        .mut_subcommand("create", |cmd| {
            cmd.mut_arg("status", |arg| {
                arg.value_parser(PossibleValuesParser::new(values.iter().copied()))
            })
        })
        .mut_subcommand("status", |cmd| {
            cmd.mut_arg("status", |arg| {
                arg.value_parser(PossibleValuesParser::new(values.iter().copied()))
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_and_doc_statuses_are_validated_before_authentication() {
        for (kind, status) in [("plan", "done"), ("doc", "outdated")] {
            assert!(
                command()
                    .try_get_matches_from(["cj", kind, "status", "12345678", status])
                    .is_ok()
            );
        }
        for (kind, status) in [
            ("plan", "completed"),
            ("doc", "archived"),
            ("plan", "current"),
            ("doc", "done"),
        ] {
            let error = command()
                .try_get_matches_from(["cj", kind, "status", "12345678", status])
                .unwrap_err()
                .to_string();
            assert!(error.contains("possible values"));
        }
        assert!(
            command()
                .try_get_matches_from(["cj", "doc", "list", "--status", "all"])
                .is_ok()
        );
        assert!(
            command()
                .try_get_matches_from([
                    "cj", "plan", "create", "--title", "Test", "--status", "current"
                ])
                .is_err()
        );
    }
    #[test]
    fn checklist_step_requires_an_explicit_state_and_positive_number() {
        for flag in ["--done", "--undone"] {
            assert!(
                command()
                    .try_get_matches_from(["cj", "plan", "step", "12345678", "1", flag])
                    .is_ok()
            );
        }
        for args in [
            vec!["cj", "plan", "step", "12345678", "0", "--done"],
            vec!["cj", "plan", "step", "12345678", "1"],
            vec!["cj", "plan", "step", "12345678", "1", "--done", "--undone"],
        ] {
            assert!(command().try_get_matches_from(args).is_err());
        }
    }
}
