mod launch;
mod monitor;
mod streams;
use super::{context, lifecycle, providers, store, text};
use anyhow::{Result, ensure};
pub use launch::launch;
use std::{io::Write, process::Stdio};

pub fn run(id: &str) -> Result<()> {
    let session = store::get(&store::open()?, id)?;
    context::authorize(&session, "guest")?;
    ensure!(
        matches!(text(&session, "status"), "starting" | "recovering")
            && session["supervisor_pid"]
                .as_u64()
                .is_none_or(|pid| pid == u64::from(std::process::id())),
        "supervisor launch is no longer authorized"
    );
    crate::watches::process::install_signals();
    let mut invocation = providers::build(&session, false)?;
    invocation
        .command
        .env("CJ_DELEGATION_ID", id)
        .env("CJ_DELEGATION_ROLE", "guest")
        .env("CJ_DELEGATION_TOKEN", text(&session, "attempt_token"))
        .env("CJ_DELEGATION_ATTEMPT", session["attempt"].to_string())
        .env("CJ_DELEGATION_CLIENT", std::env::current_exe()?)
        .env("CJ_DELEGATION_DEPTH", "1")
        .stdin(if invocation.input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    super::process::configure(&mut invocation.command);
    let mut child = match invocation.command.spawn() {
        Ok(child) => child,
        Err(error) => {
            lifecycle::finish(id, 1, &format!("cannot launch provider: {error}"), "", "")?;
            return Err(error.into());
        }
    };
    let execution = (|| -> Result<()> {
        lifecycle::attach(id, child.id())?;
        if let Some(input) = invocation.input {
            let pipe = child.stdin.take();
            std::thread::spawn(move || {
                if let Some(mut stdin) = pipe {
                    let _ = stdin.write_all(input.as_bytes());
                }
            });
        }
        let output = streams::start(id, &session, &mut child)?;
        let reason = monitor::run(id, &mut child)?;
        let code = child.wait()?.code().unwrap_or(-1);
        output.complete();
        let (stdout, stderr) = streams::tails(id, &session)?;
        lifecycle::finish(id, code, &reason, &stdout, &stderr)
    })();
    if let Err(error) = &execution {
        super::process::stop(&mut child);
        let _ = lifecycle::finish(id, 1, &format!("supervisor failed: {error}"), "", "");
    }
    execution
}
