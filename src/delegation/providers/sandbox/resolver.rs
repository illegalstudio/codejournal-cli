use anyhow::{Context, Result, ensure};
use std::{io::ErrorKind, path::Path, process::Command};

/// Keep runtime-backed DNS configuration available without exposing its siblings.
pub fn preserve(command: &mut Command) -> Result<()> {
    let path = match Path::new("/etc/resolv.conf").canonicalize() {
        Ok(path) => path,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error).context("cannot resolve read-only sandbox DNS configuration");
        }
    };
    if path.starts_with("/run") {
        ensure!(
            path.is_file(),
            "read-only sandbox DNS configuration must be a regular file: {}",
            path.display()
        );
        command.arg("--ro-bind").arg(&path).arg(&path);
    }
    Ok(())
}
