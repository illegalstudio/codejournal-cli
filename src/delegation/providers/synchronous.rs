use super::super::{args, context};
use anyhow::{Result, ensure};
use serde_json::json;
use std::{io::Write, process::Stdio};

pub fn run(args: args::StartArgs) -> Result<()> {
    context::host_only()?;
    ensure!(
        std::env::var("CJ_DELEGATION_DEPTH")
            .unwrap_or_default()
            .is_empty(),
        "nested delegation is disabled"
    );
    ensure!(
        args.mode != args::Mode::Write || args.dry_run,
        "write mode requires a durable delegation start"
    );
    ensure!(
        args.context_file.is_empty() && args.context_result.is_empty(),
        "context attachments require a durable delegation start/create"
    );
    let body = crate::input::body(args.prompt, args.prompt_file)?;
    ensure!(
        body.len() <= 60000,
        "large prompts require a durable delegation start --prompt-file PATH; the full task remains available through sync --full"
    );
    let cwd = std::env::current_dir()?.canonicalize()?;
    let session = json!({"target":args.target.name(),"mode":args.mode.name(),"cwd":cwd,"task":body,"model":args.model.unwrap_or_default()});
    if args.dry_run {
        return crate::output::json(&super::dry_run(&session)?);
    }
    super::available(args.target.name())?;
    let (invocation, mut child) = super::spawn(&session, |command, input| {
        command.env("CJ_DELEGATION_DEPTH", "1");
        command.stderr(Stdio::piped());
        if input {
            command.stdin(Stdio::piped());
        }
    })?;
    super::sandbox::notice(&invocation.sandbox);
    let errors = child.stderr.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let _ = std::io::copy(&mut pipe, &mut std::io::stderr());
        })
    });
    let writer = invocation.input.map(|input| {
        let stdin = child.stdin.take();
        std::thread::spawn(move || {
            if let Some(mut pipe) = stdin {
                let _ = pipe.write_all(input.as_bytes());
            }
        })
    });
    let status = child.wait()?;
    if let Some(errors) = errors {
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(200);
        while !errors.is_finished() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        if errors.is_finished() {
            let _ = errors.join();
        }
    }
    if let Some(writer) = writer {
        let _ = writer.join();
    }
    ensure!(status.success(), "delegated agent exited with {status}");
    Ok(())
}
