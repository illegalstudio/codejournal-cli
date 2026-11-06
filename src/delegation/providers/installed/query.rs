use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

pub fn installed(manager: &Path, name: &str, cwd: &Path) -> Option<PathBuf> {
    let mut command = Command::new(manager);
    command
        .args(["which", name])
        .current_dir(cwd)
        .env("MISE_NOT_FOUND_AUTO_INSTALL", "false")
        .env("MISE_NOT_FOUND_SYSTEM_FALLBACK", "false")
        .env("MISE_NO_ENV", "1")
        .env_remove("CJ_TOKEN")
        .env_remove("CJ_DELEGATION_TOKEN")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    super::super::super::process::configure(&mut command);
    let mut child = command.spawn().ok()?;
    let pipe = child.stdout.take()?;
    let (send, receive) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut output = Vec::new();
        if pipe.take(8193).read_to_end(&mut output).is_ok() {
            let _ = send.send(output);
        }
    });
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                super::super::super::process::stop(&mut child);
                return None;
            }
        }
    }
    let output = receive.recv_timeout(Duration::from_millis(100)).ok()?;
    if output.len() > 8192 {
        return None;
    }
    let path = PathBuf::from(String::from_utf8(output).ok()?.trim());
    if !path.is_absolute() || !path.is_file() {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if path.metadata().ok()?.permissions().mode() & 0o111 == 0 {
            return None;
        }
    }
    Some(path)
}
