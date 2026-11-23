//! Hook events queued without an account cannot be delivered by anyone. They are kept long
//! enough to be bound by hand, then removed so they stop being reported forever.
use anyhow::Result;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub const UNBOUND_HOURS: u64 = 48;
const HOUR_NANOS: u128 = 3_600_000_000_000;

/// When the event was queued, in nanoseconds since the epoch, read from its queue file name.
fn queued_at(path: &Path) -> Option<u128> {
    let (epoch, _) = path.file_name()?.to_str()?.split_once('-')?;
    (epoch.len() == 20).then(|| epoch.parse().ok()).flatten()
}

/// A file whose queue time cannot be read is never considered expired.
fn expired(path: &Path, now: u128) -> bool {
    queued_at(path)
        .is_some_and(|queued| now.saturating_sub(queued) > u128::from(UNBOUND_HOURS) * HOUR_NANOS)
}

/// Removes the unbound events queued more than 48 hours ago and returns how many they were.
/// The caller holds the hook delivery lock.
pub fn sweep() -> Result<usize> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let mut removed = 0;
    for (path, event) in super::entries()? {
        if super::origin::bound(&event).is_some() || !expired(&path, now) {
            continue;
        }
        match fs::remove_file(&path) {
            Ok(()) => removed += 1,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(removed)
}

pub fn notice(removed: usize) -> String {
    format!(
        "Removed {removed} hook event(s) that had no account binding for more than {UNBOUND_HOURS} hours."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn only_dated_files_older_than_the_window_expire() {
        let now = 1_000 * HOUR_NANOS;
        let file = |queued: u128| PathBuf::from(format!("{queued:020}-id.json"));
        assert!(expired(&file(now - 49 * HOUR_NANOS), now));
        assert!(!expired(&file(now - 47 * HOUR_NANOS), now));
        assert!(!expired(&file(now + HOUR_NANOS), now));
        assert!(!expired(
            Path::new("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa.json"),
            now
        ));
    }
}
