pub(super) mod adapters;
mod mcp_environment;
mod prompt;
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::{path::PathBuf, process::Command};

pub struct Invocation {
    pub command: Command,
    pub input: Option<String>,
    pub _config: Option<tempfile::NamedTempFile>,
}

pub fn executable(target: &str) -> &'static str {
    match target {
        "claude" => "claude",
        "codex" => "codex",
        "grok" => "grok",
        "kimi" => "kimi",
        "pi" => "pi",
        "cursor" if find("agent").is_some() => "agent",
        "cursor" => "cursor-agent",
        _ => "",
    }
}

fn find(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?).find_map(|directory| {
        let path = directory.join(name);
        #[cfg(windows)]
        let path = path.with_extension("exe");
        path.is_file().then_some(path)
    })
}

pub fn available(target: &str) -> Result<()> {
    let name = executable(target);
    ensure!(
        !name.is_empty() && find(name).is_some(),
        "target agent CLI is not installed or available in PATH: {target}"
    );
    Ok(())
}

pub fn build(session: &Value, dry: bool) -> Result<Invocation> {
    adapters::build(session, &prompt::build(session), dry)
}

pub fn dry_run(session: &Value) -> Result<Value> {
    let invocation = build(session, true)?;
    Ok(
        json!({"dry_run":true,"executable":invocation.command.get_program().to_string_lossy(),
        "args":invocation.command.get_args().map(|s|s.to_string_lossy().into_owned()).collect::<Vec<_>>(),
        "stdin":invocation.input,"cwd":session["cwd"]}),
    )
}

pub fn synchronous(args: super::args::StartArgs) -> Result<()> {
    super::context::host_only()?;
    ensure!(
        std::env::var("CJ_DELEGATION_DEPTH")
            .unwrap_or_default()
            .is_empty(),
        "nested delegation is disabled"
    );
    ensure!(
        args.mode != super::args::Mode::Write || args.dry_run,
        "write mode requires a durable delegation start"
    );
    let body = crate::input::body(args.prompt, args.prompt_file)?;
    let cwd = std::env::current_dir()?.canonicalize()?;
    let session = json!({"target":args.target.name(),"mode":args.mode.name(),"cwd":cwd,"task":body,
        "model":args.model.unwrap_or_default()});
    if args.dry_run {
        return crate::output::json(&dry_run(&session)?);
    }
    available(args.target.name())?;
    let mut invocation = build(&session, false)?;
    invocation.command.env("CJ_DELEGATION_DEPTH", "1");
    if let Some(input) = invocation.input {
        use std::io::Write;
        invocation.command.stdin(std::process::Stdio::piped());
        let mut child = invocation.command.spawn()?;
        let stdin = child.stdin.take();
        let writer = std::thread::spawn(move || {
            if let Some(mut pipe) = stdin {
                let _ = pipe.write_all(input.as_bytes());
            }
        });
        let status = child.wait()?;
        let _ = writer.join();
        ensure!(status.success(), "delegated agent exited with {status}");
    } else {
        let status = invocation.command.status()?;
        ensure!(status.success(), "delegated agent exited with {status}");
    }
    Ok(())
}
