use anyhow::Result;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn notify(cwd: &Path, agent: &str, reason: &str, approval_id: Option<&str>) -> Result<()> {
    let mut command = Command::new(std::env::current_exe()?);
    command.arg("notify");
    if let Some(id) = approval_id {
        command.args(["--hook-notification-id", id]);
    }
    command
        .current_dir(cwd)
        .args([
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
