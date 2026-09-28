use std::fs;
use std::path::Path;
use std::process::Command;

const INTERPRETERS: &[&str] = &[
    "node", "bun", "deno", "python", "python3", "bash", "sh", "zsh", "dash",
];

pub fn nearest() -> Option<&'static str> {
    if [
        "CODE_JOURNAL_NO_PROCESS_DETECT",
        "ME_AI_JOURNAL_NO_PROCESS_DETECT",
    ]
    .into_iter()
    .any(|name| std::env::var(name).as_deref() == Ok("1"))
    {
        return None;
    }
    #[cfg(unix)]
    let mut pid = unsafe { libc::getppid() };
    #[cfg(not(unix))]
    let mut pid = 0;
    for _ in 0..40 {
        if pid <= 1 {
            break;
        }
        let (parent, argv) = process(pid)?;
        if let Some(agent) = from_argv(&argv) {
            return Some(agent);
        }
        pid = parent;
    }
    None
}

fn process(pid: i32) -> Option<(i32, Vec<String>)> {
    let proc = format!("/proc/{pid}");
    if Path::new(&proc).is_dir() {
        let stat = fs::read_to_string(format!("{proc}/stat")).ok()?;
        let parent = stat
            .rsplit_once(')')?
            .1
            .split_whitespace()
            .nth(1)?
            .parse()
            .ok()?;
        let argv = fs::read(format!("{proc}/cmdline"))
            .ok()?
            .split(|byte| *byte == 0)
            .filter(|part| !part.is_empty())
            .map(|part| String::from_utf8_lossy(part).into_owned())
            .collect();
        return Some((parent, argv));
    }
    let output = Command::new("ps")
        .args(["-o", "ppid=", "-o", "command=", "-p"])
        .arg(pid.to_string())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let line = String::from_utf8_lossy(&output.stdout);
    let mut parts = line.split_whitespace();
    let parent = parts.next()?.parse().ok()?;
    Some((parent, parts.map(str::to_owned).collect()))
}

fn from_argv(argv: &[String]) -> Option<&'static str> {
    let first = argv.first()?;
    let executable = Path::new(first).file_name()?.to_str()?;
    let mut names = vec![executable];
    if INTERPRETERS.contains(&executable.split('.').next().unwrap_or(""))
        || executable.starts_with("python")
    {
        names.extend(
            argv.iter()
                .skip(1)
                .take(2)
                .filter(|part| !part.starts_with('-'))
                .filter_map(|part| Path::new(part).file_name()?.to_str()),
        );
    }
    for name in names {
        let bare = name.trim_end_matches(".js").trim_end_matches(".mjs");
        let agent = match bare {
            "claude" => "claude",
            "codex" => "codex",
            "kimi" | "kimi-code" => "kimi",
            "pi" => "pi",
            "grok" => "grok",
            "cursor-agent" => "cursor",
            "opencode" => "opencode",
            _ => continue,
        };
        return Some(agent);
    }
    first.contains("cursor-agent").then_some("cursor")
}

#[cfg(test)]
mod tests {
    use super::from_argv;

    #[test]
    fn nearest_executable_wins_over_inherited_environment() {
        assert_eq!(
            from_argv(&["/usr/bin/node".into(), "/opt/claude.js".into()]),
            Some("claude")
        );
        assert_eq!(
            from_argv(&["/usr/local/bin/kimi-code".into()]),
            Some("kimi")
        );
        assert_eq!(
            from_argv(&["/opt/cursor-agent/bin/agent".into()]),
            Some("cursor")
        );
    }
}
