use anyhow::{Result, bail};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const RETENTION_SECONDS: u64 = 90 * 86400;

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|time| time.as_secs())
        .unwrap_or(0)
}

pub fn legacy_created_at(path: &Path) -> Result<u64> {
    if let Some(seconds) = path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.split('-').next())
        .and_then(|epoch| epoch.parse::<u128>().ok())
        .and_then(|epoch| u64::try_from(epoch / 1_000_000_000).ok())
        .filter(|seconds| *seconds >= 946684800)
    {
        return Ok(seconds);
    }
    Ok(path
        .metadata()?
        .modified()?
        .duration_since(UNIX_EPOCH)?
        .as_secs())
}

pub fn ensure_retryable(created_at: Option<u64>) -> Result<()> {
    let Some(seconds) = created_at else {
        bail!("queued request creation time is unknown; inspect it before resubmitting");
    };
    if seconds > now().saturating_add(300) || now().saturating_sub(seconds) > RETENTION_SECONDS {
        bail!("safe retry window has expired; check existing records before resubmitting");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_or_unknown_writes_are_never_replayed() {
        assert!(ensure_retryable(Some(now() - RETENTION_SECONDS - 10)).is_err());
        assert!(ensure_retryable(None).is_err());
        assert!(ensure_retryable(Some(now() + 600)).is_err());
        assert!(ensure_retryable(Some(now())).is_ok());
    }
}
