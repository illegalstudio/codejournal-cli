mod paths;
use anyhow::{Context, Result};
use serde_json::Value;
use std::{path::Path, process::Command};

/// The provider and all of its shell descendants inherit the filesystem boundary.
pub fn protect(
    command: &mut Command,
    session: &Value,
    dry: bool,
) -> Result<Option<tempfile::TempDir>> {
    if super::super::text(session, "mode") != "read-only" {
        return Ok(None);
    }
    let scratch = if dry {
        None
    } else {
        Some(
            tempfile::Builder::new()
                .prefix("sandbox-")
                .tempdir_in(super::super::store::directory()?)?,
        )
    };
    let root = scratch
        .as_ref()
        .map_or(Path::new("<private scratch directory>"), |dir| dir.path());
    let writable = paths::writable(session, root, dry)?;
    let mut sandbox = wrapper(root, &writable)?;
    sandbox.arg(command.get_program()).args(command.get_args());
    for (key, value) in command.get_envs() {
        match value {
            Some(value) => {
                sandbox.env(key, value);
            }
            None => {
                sandbox.env_remove(key);
            }
        }
    }
    if let Some(cwd) = command.get_current_dir() {
        sandbox.current_dir(cwd);
    }
    for name in [
        "DBUS_SESSION_BUS_ADDRESS",
        "SSH_AUTH_SOCK",
        "DISPLAY",
        "WAYLAND_DISPLAY",
    ] {
        sandbox.env_remove(name);
    }
    sandbox
        .env("CJ_DELEGATION_SANDBOX", "1")
        .env("TMPDIR", root.join("tmp"))
        .env("XDG_CACHE_HOME", root.join("cache"))
        .env("XDG_RUNTIME_DIR", root.join("runtime"))
        .env("PYTHONDONTWRITEBYTECODE", "1");
    if !dry {
        crate::config::credential_scope::forward(&mut sandbox);
    }
    *command = sandbox;
    Ok(scratch)
}

#[cfg(target_os = "linux")]
fn wrapper(_scratch: &Path, writable: &[std::path::PathBuf]) -> Result<Command> {
    let executable = super::find("bwrap")
        .or_else(|| {
            Path::new("/usr/bin/bwrap")
                .is_file()
                .then(|| "/usr/bin/bwrap".into())
        })
        .context("read-only delegation requires bubblewrap (bwrap); no provider was launched")?;
    let mut command = Command::new(executable);
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
    for path in writable {
        command.arg("--bind").arg(path).arg(path);
    }
    command.arg("--");
    Ok(command)
}

#[cfg(target_os = "macos")]
fn wrapper(_scratch: &Path, writable: &[std::path::PathBuf]) -> Result<Command> {
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
    Ok(command)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn wrapper(_scratch: &Path, _writable: &[std::path::PathBuf]) -> Result<Command> {
    anyhow::bail!(
        "read-only delegation has no filesystem sandbox on this platform; no provider was launched"
    )
}
