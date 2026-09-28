use anyhow::{Context, Result, bail};
use std::io::{IsTerminal, Read};

pub fn body(explicit: Option<String>, file: Option<String>) -> Result<String> {
    let value = optional_body(explicit, file)?;
    if value.is_empty() {
        bail!("body required: pass --body, --body-file PATH, or pipe it on stdin");
    }
    Ok(value)
}

pub fn optional_body(explicit: Option<String>, file: Option<String>) -> Result<String> {
    if let Some(value) = explicit {
        return Ok(value.trim().to_owned());
    }
    if let Some(path) = file {
        let value = if path == "-" {
            read_stdin()
        } else {
            std::fs::read_to_string(&path).with_context(|| format!("cannot read {path}"))
        };
        return value.map(|value| value.trim().to_owned());
    }
    if !std::io::stdin().is_terminal() && stdin_ready() {
        return read_stdin().map(|value| value.trim().to_owned());
    }
    Ok(String::new())
}

#[cfg(unix)]
fn stdin_ready() -> bool {
    use std::os::fd::AsRawFd;
    let mut descriptor = libc::pollfd {
        fd: std::io::stdin().as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    unsafe { libc::poll(&mut descriptor, 1, 500) > 0 }
}

#[cfg(not(unix))]
fn stdin_ready() -> bool {
    true
}

fn read_stdin() -> Result<String> {
    let mut value = String::new();
    std::io::stdin().read_to_string(&mut value)?;
    Ok(value)
}
