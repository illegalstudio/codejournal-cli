use super::rolling::RollingLog;
use anyhow::Result;
use serde_json::Value;

mod summary;
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
            summary::collect(&value, &mut selected, 0);
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
