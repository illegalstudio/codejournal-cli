use anyhow::Result;
use rusqlite::ErrorCode;
use std::time::{Duration, Instant};

pub fn is_busy(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<rusqlite::Error>()
            .is_some_and(|error| {
                matches!(
                    error.sqlite_error_code(),
                    Some(ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked)
                )
            })
    })
}

pub fn retry<T>(action: &str, mut operation: impl FnMut() -> Result<T>) -> Result<T> {
    let started = Instant::now();
    let mut pause = Duration::from_millis(10 + u64::from(std::process::id() % 17));
    loop {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error)
                if is_busy(&error)
                    && started.elapsed() < Duration::from_secs(30)
                    && !crate::watches::process::stopped() =>
            {
                std::thread::sleep(pause);
                pause = (pause * 2).min(Duration::from_millis(200));
            }
            Err(error) => return Err(error.context(format!("delegation database: {action}"))),
        }
    }
}
