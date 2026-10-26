use std::process::{Child, Command};

pub fn identity(pid: u32) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        let rest = stat.get(stat.rfind(')')? + 2..)?;
        let fields = rest.split_whitespace().collect::<Vec<_>>();
        if fields
            .first()
            .is_some_and(|state| matches!(*state, "Z" | "X"))
        {
            return None;
        }
        return fields.get(19).map(|value| value.to_string());
    }
    #[cfg(all(unix, not(target_os = "linux")))]
    {
        let output = Command::new("/bin/ps")
            .args(["-o", "lstart=", "-p", &pid.to_string()])
            .output()
            .ok()?;
        let value = String::from_utf8(output.stdout).ok()?.trim().to_owned();
        return (!value.is_empty()).then_some(value);
    }
    #[cfg(windows)]
    {
        let query = format!(
            "(Get-CimInstance Win32_Process -Filter 'ProcessId = {pid}').CreationDate.ToFileTimeUtc()"
        );
        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &query])
            .output()
            .ok()?;
        let value = String::from_utf8(output.stdout).ok()?.trim().to_owned();
        return (output.status.success() && !value.is_empty()).then_some(value);
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = pid;
        None
    }
}

pub fn alive(value: &serde_json::Value) -> bool {
    value
        .as_u64()
        .and_then(|pid| u32::try_from(pid).ok())
        .and_then(identity)
        .is_some()
}

pub fn terminate(pid: u32, expected: &str) -> bool {
    if expected.is_empty() || identity(pid).as_deref() != Some(expected) {
        return false;
    }
    #[cfg(unix)]
    unsafe {
        return libc::kill(pid as i32, libc::SIGTERM) == 0;
    }
    #[cfg(windows)]
    {
        return Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status()
            .is_ok_and(|status| status.success());
    }
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

pub fn stop(child: &mut Child) {
    if child.try_wait().is_ok_and(|status| status.is_none()) {
        let _ = crate::watches::process::kill_tree(child);
    }
}

pub fn matches(pid: &serde_json::Value, expected: &serde_json::Value) -> bool {
    let actual = pid
        .as_u64()
        .and_then(|p| u32::try_from(p).ok())
        .and_then(identity);
    actual.is_some()
        && expected
            .as_str()
            .is_none_or(|s| actual.as_deref() == Some(s))
}

pub fn owns_processes(session: &serde_json::Value) -> bool {
    matches(&session["supervisor_pid"], &session["supervisor_identity"])
        || matches(&session["guest_pid"], &session["guest_identity"])
}

pub fn terminate_group(pid: u32, expected: &str) -> bool {
    if expected.is_empty() || identity(pid).as_deref() != Some(expected) {
        return false;
    }
    #[cfg(unix)]
    unsafe {
        return libc::kill(-(pid as i32), libc::SIGTERM) == 0;
    }
    #[cfg(not(unix))]
    {
        terminate(pid, expected)
    }
}

pub fn configure(command: &mut Command) {
    crate::watches::process::group(command);
}
