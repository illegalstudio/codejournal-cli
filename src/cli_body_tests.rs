use crate::cli::Cli;
use clap::Parser;

#[test]
fn markdown_body_arguments_accept_leading_lists() {
    for prefix in [
        vec!["add", "--kind", "howto"],
        vec!["answer", "01234567"],
        vec!["plan", "create"],
        vec!["doc", "create"],
        vec!["task", "add"],
        vec!["log", "add"],
        vec!["notify"],
        vec!["feedback", "add", "--category", "workflow"],
    ] {
        let mut args = vec!["cj"];
        args.extend(prefix);
        args.extend(["--title", "Checklist", "--body", "- [ ] Validate"]);
        assert!(Cli::try_parse_from(&args).is_ok(), "{args:?}");
    }
    for noun in ["plan", "doc"] {
        let args = [
            "cj",
            noun,
            "update",
            "01234567",
            "--body",
            "- [x] Validated",
        ];
        assert!(Cli::try_parse_from(args).is_ok());
    }
    assert!(Cli::try_parse_from(["cj", "rules", "set", "--body", "- Keep data synthetic"]).is_ok());
}
