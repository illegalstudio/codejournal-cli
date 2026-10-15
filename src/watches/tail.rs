use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const BYTES: u64 = 16_000;

pub fn regular_file(path: &Path) -> Result<File> {
    ensure!(
        std::fs::symlink_metadata(path)?.is_file(),
        "not a regular watch file"
    );
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path)?;
    ensure!(file.metadata()?.is_file(), "not a regular watch file");
    Ok(file)
}

pub fn read(path: &Path, lines: usize) -> Result<Value> {
    let mut file = regular_file(path)?;
    let metadata = file.metadata()?;
    let length = metadata.len();
    file.seek(SeekFrom::Start(length.saturating_sub(BYTES)))?;
    let mut bytes = Vec::new();
    file.take(length.min(BYTES)).read_to_end(&mut bytes)?;
    let text = String::from_utf8_lossy(&bytes);
    let mut result = bounded(&text, lines);
    result["bytes"] = json!(length);
    result["last_output_at"] = json!(
        metadata
            .modified()
            .ok()
            .map(|time| { chrono::DateTime::<chrono::Utc>::from(time).to_rfc3339() })
    );
    if length > BYTES {
        result["truncated"] = json!(true);
    }
    Ok(result)
}

pub fn bounded(text: &str, lines: usize) -> Value {
    let selected: Vec<_> = text.lines().rev().take(lines).collect();
    let joined = selected.into_iter().rev().collect::<Vec<_>>().join("\n");
    let tail: String = joined
        .chars()
        .rev()
        .take(10_000)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let truncated = text.lines().count() > lines || joined.chars().count() > 10_000;
    json!({"tail": tail, "truncated": truncated})
}
