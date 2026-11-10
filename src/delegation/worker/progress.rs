use super::rolling::RollingLog;
use anyhow::Result;
use serde_json::Value;

mod summary;
use std::path::PathBuf;

const LINE_LIMIT: usize = 256 * 1024;

/// Readable per-event summary of provider output. Every line starts with a UTC timestamp, and
/// events that carry nothing beyond `type=` stream markers are skipped so the latest tool,
/// command or status stays visible in a bounded tail.
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
        if selected.iter().any(|item| !item.starts_with("type=")) {
            self.log
                .write(format!("{} {}\n", timestamp(), selected.join(" | ")).as_bytes())?;
        }
        self.pending.clear();
        self.truncated = false;
        Ok(())
    }
}

pub const TIMESTAMP_FORMAT: &str = "%Y-%m-%dT%H:%M:%SZ";

fn timestamp() -> String {
    chrono::Utc::now().format(TIMESTAMP_FORMAT).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standalone_stream_markers_are_skipped_and_lines_carry_timestamps() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("attempt.progress.log");
        let mut progress = Progress::new(path.clone()).unwrap();
        progress
            .feed(b"{\"type\":\"tool_call\",\"name\":\"Bash\",\"input\":{\"command\":\"cargo test\"}}\n")
            .unwrap();
        for _ in 0..3 {
            progress.feed(b"{\"type\":\"stream_event\",\"event\":{\"type\":\"content_block_delta\",\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\"}}}\n").unwrap();
        }
        progress
            .feed(b"{\"type\":\"stream_event\",\"event\":{\"type\":\"content_block_stop\",\"index\":0}}\n")
            .unwrap();
        progress
            .feed(b"{\"type\":\"system\",\"subtype\":\"init\"}\n")
            .unwrap();
        let text = std::fs::read_to_string(path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 1, "{text}");
        let (stamp, summary) = lines[0].split_once(' ').unwrap();
        assert!(chrono::NaiveDateTime::parse_from_str(stamp, TIMESTAMP_FORMAT).is_ok());
        assert_eq!(summary, "name=Bash | command=cargo test");
    }
}
