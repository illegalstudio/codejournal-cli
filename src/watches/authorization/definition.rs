use super::Authorization;
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

pub fn verify(authorization: &Authorization, watch: &Value, id: &str) -> Result<()> {
    let remote: Value = serde_json::from_str(
        watch["command"]
            .as_str()
            .context("remote watch command is missing")?,
    )
    .context("remote watch command is not a JSON argument array")?;
    let expected = crate::api::sanitized(json!(authorization.command));
    let mut fields = Vec::new();
    if watch["id"] != id {
        fields.push("id".to_owned());
    }
    if remote != expected {
        match (remote.as_array(), expected.as_array()) {
            (Some(actual), Some(expected)) => {
                if actual.len() != expected.len() {
                    fields.push(format!(
                        "command argument count (expected {}, received {})",
                        expected.len(),
                        actual.len()
                    ));
                }
                for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
                    if actual != expected {
                        fields.push(format!("command[{index}]"));
                    }
                }
            }
            _ => fields.push("command argument array".to_owned()),
        }
    }
    if watch["cwd"] != crate::api::sanitized(json!(authorization.cwd)) {
        fields.push("cwd".to_owned());
    }
    if watch["timeout"] != json!(authorization.timeout) {
        fields.push("timeout".to_owned());
    }
    if !fields.is_empty() {
        bail!(
            "remote watch definition differs from the locally authorized command: {}. Execution refused; argument values are withheld",
            fields.join(", ")
        );
    }
    Ok(())
}
