use crate::cli::Cli;
use clap::builder::PossibleValuesParser;
use clap::{Command, CommandFactory, FromArgMatches};

pub fn parse() -> Cli {
    let matches = command().try_get_matches().unwrap_or_else(|error| {
        let args: Vec<String> = std::env::args().skip(1).collect();
        if let Some(hint) = body_hint(&error, &args) {
            let _ = error.print();
            eprintln!("\n{hint}");
            std::process::exit(error.exit_code());
        }
        error.exit()
    });
    Cli::from_arg_matches(&matches).unwrap_or_else(|error| error.exit())
}

/// Text passed without a flag to a command that takes it through `--body`.
fn body_hint(error: &clap::Error, args: &[String]) -> Option<String> {
    use clap::error::{ContextKind, ContextValue, ErrorKind};
    if error.kind() != ErrorKind::UnknownArgument {
        return None;
    }
    let Some(ContextValue::String(unexpected)) = error.get(ContextKind::InvalidArg) else {
        return None;
    };
    let root = command();
    let mut current = &root;
    for arg in args {
        match current.find_subcommand(arg) {
            Some(next) => current = next,
            None if current.has_subcommands() && arg.starts_with('-') => {}
            None => break,
        }
    }
    let takes_body = current
        .get_arguments()
        .any(|arg| arg.get_long() == Some("body"));
    (takes_body && !unexpected.starts_with('-')).then(|| {
        "hint: this command takes no positional text; pass it with --body \"...\" or --body-file FILE".to_owned()
    })
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
    fn positional_text_points_to_the_body_flag() {
        let hint = |args: &[&str]| {
            let error = command()
                .try_get_matches_from([&["cj"], args].concat())
                .unwrap_err();
            let args: Vec<String> = args.iter().map(|arg| (*arg).to_owned()).collect();
            body_hint(&error, &args)
        };
        assert!(hint(&["log", "add", "--title", "Done", "What changed"]).is_some());
        assert!(hint(&["--json", "feedback", "add", "It failed"]).is_some());
        assert!(hint(&["log", "add", "--titel", "Done"]).is_none());
        assert!(hint(&["supersede", "12345678", "extra"]).is_none());
        assert!(hint(&["log", "add"]).is_none());
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
