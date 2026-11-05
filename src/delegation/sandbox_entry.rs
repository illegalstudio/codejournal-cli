use anyhow::{Context, Result};
use std::{ffi::OsStr, process::Command};

/// The marker is written immediately before exec so a provider is never retried after it starts.
pub fn run() -> Result<bool> {
    let mut args = std::env::args_os().skip(1);
    let operation = args.next();
    if operation.as_deref() == Some(OsStr::new("__cj_bundled_bwrap_info")) {
        crate::output::json(&super::bundled::info())?;
        return Ok(true);
    }
    if operation.as_deref() != Some(OsStr::new("__cj_sandbox_exec")) {
        return Ok(false);
    }
    let marker = args.next().context("missing sandbox launch marker")?;
    let program = args.next().context("missing sandbox provider executable")?;
    std::fs::write(marker, b"started")?;
    let mut command = Command::new(program);
    command.args(args);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        Err(command.exec()).context("cannot execute sandboxed provider")
    }
    #[cfg(not(unix))]
    {
        let status = command.status()?;
        std::process::exit(status.code().unwrap_or(1));
    }
}
