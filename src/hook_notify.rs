use anyhow::Result;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn notify(cwd: &Path, agent: &str, reason: &str) -> Result<()> {
    Command::new(std::env::current_exe()?)
        .current_dir(cwd)
        .args([
            "notify",
            "--kind",
            "needs_input",
            "--agent",
            agent,
            "--title",
            &format!(
                "{agent} is waiting: {}",
                reason.chars().take(120).collect::<String>()
            ),
            "--body",
            "Answer in the agent's session.",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}
