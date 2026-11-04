use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::{
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

pub fn read(path: &Path, lines: u16, max_chars: u32, raw: bool) -> Result<Value> {
    regular(path)?;
    let summary = path.with_extension("progress.log");
    let readable = !raw && summary.metadata().is_ok_and(|metadata| metadata.len() > 0);
    let selected_path = if readable { &summary } else { path };
    regular(selected_path)?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let mut file = match options.open(selected_path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(json!({"available":false,"text":""}));
        }
        Err(error) => return Err(error.into()),
    };
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file(),
        "diagnostic output must be a regular file"
    );
    let size = metadata.len();
    let budget = u64::from(max_chars).saturating_mul(4).max(16384);
    file.seek(SeekFrom::Start(size.saturating_sub(budget)))?;
    let mut bytes = Vec::new();
    file.take(budget).read_to_end(&mut bytes)?;
    let content = String::from_utf8_lossy(&bytes);
    let mut selected = content
        .lines()
        .rev()
        .take(usize::from(lines))
        .collect::<Vec<_>>();
    selected.reverse();
    let text = selected.join("\n");
    let length = text.chars().count();
    let bounded = text
        .chars()
        .skip(length.saturating_sub(max_chars as usize))
        .collect::<String>();
    Ok(
        json!({"available":true,"text":bounded,"format":if readable {"summary"} else {"raw"},
        "capture_truncated":path.with_extension("truncated").exists() || path.metadata()?.len() >= 2097152,
        "tail_truncated":size > bytes.len() as u64 || content.lines().count() > usize::from(lines) || length > max_chars as usize}),
    )
}

fn regular(path: &Path) -> Result<()> {
    if let Ok(metadata) = path.symlink_metadata() {
        ensure!(
            metadata.is_file() && !metadata.file_type().is_symlink(),
            "diagnostic output must be a regular file"
        );
    }
    Ok(())
}
