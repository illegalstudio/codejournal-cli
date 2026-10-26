use anyhow::{Context, Result};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};

static STOP: AtomicBool = AtomicBool::new(false);

pub fn detached(command: &mut Command) {
    #[cfg(unix)]
    unsafe {
        use std::os::unix::process::CommandExt;
        command.pre_exec(|| {
            if libc::setsid() < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x00000008 | 0x00000200);
    }
}

pub fn install_signals() {
    #[cfg(unix)]
    unsafe {
        extern "C" fn stop(_: libc::c_int) {
            STOP.store(true, Ordering::Relaxed);
        }
        libc::signal(libc::SIGTERM, stop as *const () as libc::sighandler_t);
        libc::signal(libc::SIGINT, stop as *const () as libc::sighandler_t);
    }
}

pub fn stopped() -> bool {
    STOP.load(Ordering::Relaxed)
}

pub fn group(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x00000200);
    }
}

pub fn kill_tree(child: &mut Child) -> Result<()> {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .status();
    }
    let _ = child.kill();
    child.wait().context("cannot reap watch command")?;
    Ok(())
}

pub fn owns(pid: u32, id: &str) -> bool {
    #[cfg(target_os = "linux")]
    {
        let Ok(bytes) = std::fs::read(format!("/proc/{pid}/cmdline")) else {
            return false;
        };
        let args = bytes
            .split(|byte| *byte == 0)
            .filter_map(|arg| std::str::from_utf8(arg).ok())
            .collect::<Vec<_>>();
        return args.windows(3).any(|args| args == ["watch", "run", id])
            || args.windows(2).any(|args| args == ["watch-run", id]);
    }
    #[cfg(all(unix, not(target_os = "linux")))]
    {
        let Ok(output) = Command::new("ps")
            .args(["-o", "command=", "-p", &pid.to_string()])
            .output()
        else {
            return false;
        };
        let text = String::from_utf8_lossy(&output.stdout);
        let args = text.split_whitespace().collect::<Vec<_>>();
        args.windows(3).any(|args| args == ["watch", "run", id])
            || args.windows(2).any(|args| args == ["watch-run", id])
    }
    #[cfg(windows)]
    {
        let query = format!(
            "Get-CimInstance Win32_Process -Filter 'ProcessId = {pid}' | Select-Object Name,CommandLine | ConvertTo-Json -Compress"
        );
        let Ok(output) = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &query])
            .output()
        else {
            return false;
        };
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
            return false;
        };
        if value["Name"].as_str() != Some("cj.exe") {
            return false;
        }
        let args = value["CommandLine"]
            .as_str()
            .unwrap_or("")
            .split_whitespace()
            .map(|argument| argument.trim_matches('"'))
            .collect::<Vec<_>>();
        args.windows(3).any(|args| args == ["watch", "run", id])
            || args.windows(2).any(|args| args == ["watch-run", id])
    }
}

pub fn terminate(pid: u32, id: &str) -> bool {
    if !owns(pid, id) {
        return false;
    }
    #[cfg(unix)]
    unsafe {
        return libc::kill(pid as i32, libc::SIGTERM) == 0;
    }
    #[cfg(windows)]
    {
        Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status()
            .is_ok_and(|status| status.success())
    }
}
