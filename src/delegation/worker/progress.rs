use super::rolling::RollingLog;
use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;

const LINE_LIMIT: usize = 256 * 1024;

pub struct Progress {
    log: RollingLog,
    pending: Vec<u8>,
    truncated: bool,
}

impl Progress {
    pub fn new(path: PathBuf) -> Result<Self> {
        Ok(Self {
            log: RollingLog::new(path)?,
            pending: Vec::new(),
            truncated: false,
        })
    }

    pub fn feed(&mut self, bytes: &[u8]) -> Result<()> {
        for part in bytes.split_inclusive(|byte| *byte == b'\n') {
            let amount = part
                .len()
                .min(LINE_LIMIT.saturating_sub(self.pending.len()));
            self.pending.extend_from_slice(&part[..amount]);
            self.truncated |= amount < part.len();
            if part.last() == Some(&b'\n') {
                self.finish()?;
            }
        }
        Ok(())
    }

    pub fn finish(&mut self) -> Result<()> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let mut selected = Vec::new();
        if let Ok(value) = serde_json::from_slice::<Value>(&self.pending) {
            summarize(&value, &mut selected, 0);
        } else if self.truncated {
            let text = String::from_utf8_lossy(&self.pending);
            let pattern =
                regex::Regex::new(r#""(type|name|tool_name|status)"\s*:\s*"([^"\n]{1,120})""#)?;
            selected.extend(
                pattern
                    .captures_iter(&text)
                    .take(12)
                    .map(|m| format!("{}={}", &m[1], &m[2])),
            );
            selected.push(
                "[large provider event summarized; use --raw for bounded raw output]".to_owned(),
            );
        } else {
            selected.push(
                String::from_utf8_lossy(&self.pending)
                    .trim()
                    .chars()
                    .take(800)
                    .collect(),
            );
        }
        if !selected.is_empty() {
            self.log
                .write(format!("{}\n", selected.join(" | ")).as_bytes())?;
        }
        self.pending.clear();
        self.truncated = false;
        Ok(())
    }
}

fn summarize(value: &Value, selected: &mut Vec<String>, depth: usize) {
    if depth > 10 || selected.len() >= 12 {
        return;
    }
    match value {
        Value::Object(map) => {
            if map.get("type").and_then(Value::as_str).is_some_and(|kind| {
                matches!(
                    kind,
                    "thinking_delta"
                        | "signature_delta"
                        | "input_json_delta"
                        | "usage"
                        | "usage_update"
                )
            }) || map
                .get("delta")
                .and_then(|delta| delta.get("type"))
                .and_then(Value::as_str)
                .is_some_and(|kind| {
                    matches!(
                        kind,
                        "thinking_delta" | "signature_delta" | "input_json_delta"
                    )
                })
            {
                return;
            }
            for name in ["type", "name", "tool_name", "status", "text", "message"] {
                if let Some(text) = map.get(name).and_then(Value::as_str) {
                    selected.push(format!(
                        "{name}={}",
                        text.chars()
                            .take(400)
                            .collect::<String>()
                            .replace('\n', " ")
                    ));
                    if selected.len() >= 12 {
                        return;
                    }
                }
            }
            for (name, child) in map {
                if !matches!(
                    name.as_str(),
                    "rawOutput" | "stdout" | "stderr" | "input" | "arguments" | "data" | "bytes"
                ) {
                    summarize(child, selected, depth + 1);
                }
            }
        }
        Value::Array(items)
            if items
                .first()
                .is_some_and(|item| item.is_object() || item.is_array()) =>
        {
            for item in items.iter().take(16) {
                summarize(item, selected, depth + 1);
            }
        }
        _ => {}
    }
}
