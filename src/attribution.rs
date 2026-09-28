use std::process::Command;

fn variable(name: &str) -> Option<String> {
    std::env::var(format!("CODE_JOURNAL_{name}"))
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            std::env::var(format!("ME_AI_JOURNAL_{name}"))
                .ok()
                .filter(|value| !value.is_empty())
        })
}

pub fn agent(explicit: Option<&str>) -> String {
    explicit
        .map(str::to_owned)
        .or_else(|| variable("AGENT"))
        .or_else(|| crate::agent_process::nearest().map(str::to_owned))
        .or_else(|| {
            std::env::var("CLAUDECODE")
                .ok()
                .map(|_| "claude".to_owned())
        })
        .or_else(|| {
            std::env::var("CURSOR_AGENT")
                .ok()
                .map(|_| "cursor".to_owned())
        })
        .or_else(|| {
            (std::env::var("CODEX_SANDBOX").is_ok() || std::env::var("CODEX_THREAD_ID").is_ok())
                .then(|| "codex".to_owned())
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

pub fn host() -> String {
    variable("HOST")
        .or_else(|| {
            Command::new("hostname")
                .output()
                .ok()
                .filter(|result| result.status.success())
                .map(|result| String::from_utf8_lossy(&result.stdout).trim().to_owned())
        })
        .unwrap_or_else(|| "unknown".to_owned())
}
