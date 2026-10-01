use base64::{Engine, engine::general_purpose::STANDARD};
use std::path::Path;

const MARKER: &str = "CJ_RUST_HOOK=1";
const WINDOWS_PREFIX: &str = "powershell.exe -NoProfile -NonInteractive -EncodedCommand ";
const WINDOWS_MARKER: &str = "$env:CJ_RUST_HOOK='1'; ";

pub fn build(binary: &Path, event: &str) -> String {
    if cfg!(windows) {
        windows(binary, event)
    } else {
        format!(
            "{MARKER} '{}' hook {event}",
            binary.to_string_lossy().replace('\'', "'\\''")
        )
    }
}

fn windows(binary: &Path, event: &str) -> String {
    let raw = binary.to_string_lossy();
    let path = if let Some(unc) = raw.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        raw.strip_prefix(r"\\?\").unwrap_or(&raw).to_owned()
    };
    let script = format!(
        "{WINDOWS_MARKER}& '{}' hook {event}; exit $LASTEXITCODE",
        path.replace('\'', "''")
    );
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    format!("{WINDOWS_PREFIX}{}", STANDARD.encode(bytes))
}

pub fn owned(command: &str) -> bool {
    if command.contains(MARKER) {
        return true;
    }
    let Some(encoded) = command.strip_prefix(WINDOWS_PREFIX) else {
        return false;
    };
    let Ok(bytes) = STANDARD.decode(encoded) else {
        return false;
    };
    if bytes.len() % 2 != 0 {
        return false;
    }
    let words: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|part| u16::from_le_bytes([part[0], part[1]]))
        .collect();
    String::from_utf16(&words).is_ok_and(|script| script.starts_with(WINDOWS_MARKER))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoded_windows_hooks_are_safe_and_removable() {
        let command = windows(Path::new(r"\\?\C:\Tool's Directory\cj.exe"), "SessionStart");
        assert!(owned(&command));
        let encoded = command.strip_prefix(WINDOWS_PREFIX).unwrap();
        let bytes = STANDARD.decode(encoded).unwrap();
        let words: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|part| u16::from_le_bytes([part[0], part[1]]))
            .collect();
        let script = String::from_utf16(&words).unwrap();
        assert!(script.contains("& 'C:\\Tool''s Directory\\cj.exe' hook SessionStart"));
        assert!(!owned(&format!("{WINDOWS_PREFIX}broken")));
        assert!(!owned("echo unrelated hook"));
    }
}
