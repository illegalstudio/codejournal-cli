use crate::{api::Api, project_bootstrap};
use anyhow::{Context, Result, bail};
use clap::Args;
use std::io::{IsTerminal, Write};
use std::process::{Command, Stdio};

#[derive(Args)]
pub struct BrowseArgs {
    pub slug: Option<String>,
    #[arg(long)]
    pub all_projects: bool,
    #[arg(long)]
    pub all_statuses: bool,
    #[arg(long)]
    pub list: bool,
    #[arg(long)]
    pub list_projects: bool,
}

pub fn run(api: &Api, tenant: &str, current: Option<&str>, args: BrowseArgs) -> Result<()> {
    if args.list_projects {
        return crate::browse_list::projects(api, tenant);
    }
    let interactive = !args.list;
    if interactive && (!std::io::stdin().is_terminal() || !std::io::stdout().is_terminal()) {
        bail!("browse is interactive; run it from a terminal, not through an agent");
    }
    let picked = if interactive && args.slug.is_none() && current.is_none() && !args.all_projects {
        match crate::browse_picker::choose(api, tenant)? {
            Some(value) => Some(value),
            None => return Ok(()),
        }
    } else {
        None
    };
    let slug = args.slug.as_deref().or(current).or(picked.as_deref());
    let scope = if args.all_projects || slug == Some("*") {
        None
    } else {
        Some(project_bootstrap::resolved_slug(api, tenant, slug)?)
    };
    let mut lines = crate::browse_list::entries(api, tenant, scope.as_deref(), args.all_statuses)?;
    if args.list {
        for line in lines {
            println!("{line}");
        }
        return Ok(());
    }
    if lines.is_empty() && scope.is_some() {
        eprintln!("No entries in this project; showing all projects.");
        lines = crate::browse_list::entries(api, tenant, None, args.all_statuses)?;
    }
    if lines.is_empty() {
        println!("The journal is empty.");
        return Ok(());
    }
    let executable = std::env::current_exe()?;
    let binary = format!(
        "'{}'",
        executable.display().to_string().replace("'", "'\\''")
    );
    let statuses = if args.all_statuses {
        " --all-statuses"
    } else {
        ""
    };
    let preview = format!("{binary} show --no-track {{1}}");
    let all = format!(
        "ctrl-a:reload({binary} browse --list --all-projects{statuses})+change-prompt(all> )"
    );
    let current = format!(
        "ctrl-p:reload({binary} browse --list {{2}}{statuses})+transform-prompt(echo {{2}}'> ')"
    );
    let rules = format!("ctrl-r:execute({binary} --project {{2}} rules)");
    let mut process = Command::new("fzf")
        .args([
            "--ansi",
            "--delimiter",
            "\t",
            "--with-nth",
            "2..",
            "--tiebreak",
            "index",
            "--prompt",
            "journal> ",
            "--header",
            "type to filter | enter: open | ctrl-a: all projects | ctrl-p: this entry's project | ctrl-r: rules | esc: quit",
            "--preview",
            &preview,
            "--preview-window",
            "right,55%,wrap",
            "--bind",
            &all,
            "--bind",
            &current,
            "--bind",
            &rules,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("browse needs fzf on PATH")?;
    if let Some(mut stdin) = process.stdin.take() {
        stdin.write_all(lines.join("\n").as_bytes())?;
    }
    let output = process.wait_with_output()?;
    if !output.status.success() {
        return Ok(());
    }
    let selected = String::from_utf8(output.stdout)?;
    let Some(id) = selected
        .split('\t')
        .next()
        .filter(|id| uuid::Uuid::parse_str(id).is_ok())
    else {
        return Ok(());
    };
    Command::new(std::env::current_exe()?)
        .args(["show", id])
        .status()
        .context("cannot open selected entry")?;
    Ok(())
}
