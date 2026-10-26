mod launch;
mod monitor;
mod streams;
use super::{context, lifecycle, providers, store, text};
use anyhow::{Result, ensure};
pub use launch::{launch, launch_with_timeout};
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
        let status = child.wait()?;
        let code = status.code().unwrap_or(-1);
        let reason = termination(status, reason);
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

fn termination(status: std::process::ExitStatus, reason: String) -> String {
    if !reason.is_empty() {
        return reason;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return format!(
                "provider terminated by signal {signal}; the external sender is unknown"
            );
        }
    }
    if status.code().is_some_and(|code| code >= 128) {
        return format!(
            "provider exited with code {}; this may encode a signal, but its cause is unknown",
            status.code().unwrap_or(-1)
        );
    }
    String::new()
}
