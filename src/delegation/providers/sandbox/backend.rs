use anyhow::Result;
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub fn name() -> &'static str {
    if cfg!(target_os = "linux") {
        "bubblewrap"
    } else {
        "sandbox-exec"
    }
}

#[cfg(target_os = "linux")]
pub fn select(
    _root: &Path,
    writable: &[PathBuf],
    session: &Value,
    dry: bool,
    skip_system: bool,
) -> Result<(Command, &'static str)> {
    let mut failures = Vec::new();
    if !skip_system {
        let executable = super::super::find("bwrap").or_else(|| {
            Path::new("/usr/bin/bwrap")
                .is_file()
                .then(|| "/usr/bin/bwrap".into())
        });
        if let Some(path) = executable {
            match candidate(&path, writable, session, dry) {
                Ok(command) => return Ok((command, "system")),
                Err(error) => failures.push(format!("system: {error:#}")),
            }
        } else {
            failures.push("system: bubblewrap (bwrap) is not installed".to_owned());
        }
    }
    let bundled = super::super::super::bundled::extract(dry)
        .and_then(|path| candidate(&path, writable, session, dry));
    match bundled {
        Ok(command) => Ok((command, "bundled")),
        Err(error) => {
            failures.push(format!("bundled: {error:#}"));
            anyhow::bail!("no usable filesystem sandbox: {}", failures.join("; "));
        }
    }
}

#[cfg(target_os = "linux")]
fn candidate(path: &Path, writable: &[PathBuf], session: &Value, dry: bool) -> Result<Command> {
    let mut command = Command::new(path);
    command.args([
        "--unshare-user",
        "--unshare-pid",
        "--die-with-parent",
        "--ro-bind",
        "/",
        "/",
        "--dev",
        "/dev",
        "--proc",
        "/proc",
        "--tmpfs",
        "/run",
    ]);
    super::resolver::preserve(&mut command)?;
    for path in writable {
        command.arg("--bind").arg(path).arg(path);
    }
    command.arg("--");
    if !dry {
        super::probe::check(&command, session)?;
    }
    Ok(command)
}

#[cfg(target_os = "macos")]
pub fn select(
    _root: &Path,
    writable: &[PathBuf],
    session: &Value,
    dry: bool,
    _skip_system: bool,
) -> Result<(Command, &'static str)> {
    let mut profile =
        "(version 1)(allow default)(deny file-write*)(allow file-write* (literal \"/dev/null\"))"
            .to_owned();
    for path in writable {
        profile.push_str(&format!(
            "(allow file-write* (subpath {}))",
            serde_json::to_string(&path.to_string_lossy())?
        ));
    }
    let mut command = Command::new("/usr/bin/sandbox-exec");
    command.args(["-p", &profile]);
    if !dry {
        super::probe::check(&command, session)?;
    }
    Ok((command, "system"))
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn select(
    _root: &Path,
    _writable: &[PathBuf],
    _session: &Value,
    _dry: bool,
    _skip_system: bool,
) -> Result<(Command, &'static str)> {
    anyhow::bail!("no supported filesystem sandbox on this platform")
}
