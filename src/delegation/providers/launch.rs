use super::{Invocation, sandbox};
use anyhow::Result;
use serde_json::Value;
use std::{
    io::Read,
    process::{Child, Command},
    time::{Duration, Instant},
};

pub fn spawn(
    session: &Value,
    configure: impl Fn(&mut Command, bool),
) -> Result<(Invocation, Child)> {
    let mut invocation = super::build(session, false)?;
    for _ in 0..3 {
        configure(&mut invocation.command, invocation.input.is_some());
        let launched = invocation.command.spawn();
        let failure = match launched {
            Ok(mut child) => {
                if let Some(reason) = startup_failure(&invocation, &mut child)? {
                    reason
                } else {
                    return Ok((invocation, child));
                }
            }
            Err(error) if invocation.sandbox["enforced"] == true => {
                format!("cannot launch sandbox: {error}")
            }
            Err(error) => return Err(error.into()),
        };
        let protection = sandbox::retry(session, &invocation.sandbox, &failure);
        invocation = super::build_with_protection(session, false, protection)?;
    }
    anyhow::bail!("sandbox fallback did not reach a provider launch")
}

fn startup_failure(invocation: &Invocation, child: &mut Child) -> Result<Option<String>> {
    let Some(marker) = &invocation.marker else {
        return Ok(None);
    };
    let started = || {
        marker
            .as_file()
            .metadata()
            .is_ok_and(|metadata| metadata.len() > 0)
    };
    let deadline = Instant::now() + Duration::from_millis(1000);
    loop {
        if started() {
            return Ok(None);
        }
        if let Some(status) = child.try_wait()? {
            if started() {
                return Ok(None);
            }
            return Ok(Some(format!(
                "sandbox exited before starting the provider ({status}): {}",
                diagnostics(child)
            )));
        }
        if Instant::now() >= deadline {
            super::super::process::stop(child);
            return Ok((!started())
                .then(|| "sandbox startup timed out before provider execution".to_owned()));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn diagnostics(child: &mut Child) -> String {
    let Some(mut pipe) = child.stderr.take() else {
        return String::new();
    };
    #[cfg(unix)]
    unsafe {
        use std::os::fd::AsRawFd;
        let flags = libc::fcntl(pipe.as_raw_fd(), libc::F_GETFL);
        if flags < 0 || libc::fcntl(pipe.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) < 0 {
            return String::new();
        }
    }
    let mut bytes = [0_u8; 2048];
    let size = pipe.read(&mut bytes).unwrap_or(0);
    String::from_utf8_lossy(&bytes[..size]).trim().to_owned()
}
