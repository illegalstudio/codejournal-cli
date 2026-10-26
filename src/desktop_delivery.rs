use std::process::{Command, Stdio};

pub fn send(title: &str, message: &str, urgency: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let quote = |value: &str| {
            value
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
        };
        let script = format!(
            "display notification \"{}\" with title \"{}\"",
            quote(message),
            quote(title)
        );
        let mut process = Command::new("osascript");
        process.args(["-e", &script]);
        process
    };
    #[cfg(not(target_os = "macos"))]
    let mut command = {
        let mut process = Command::new("notify-send");
        process.args(["-a", "Code Journal", "-u", urgency, title, message]);
        process
    };
    match command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(format!("desktop delivery failed with exit status {status}")),
        Err(error) => Err(format!("desktop delivery failed: {error}")),
    }
}
