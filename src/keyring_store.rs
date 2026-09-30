use keyring::{Entry, Error, Result};
use std::time::Duration;

fn initialize() -> Result<()> {
    if keyring::get_default_store().is_some() {
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    let store = apple_native_keyring_store::keychain::Store::new()?;
    #[cfg(target_os = "windows")]
    let store = windows_native_keyring_store::Store::new()?;
    #[cfg(all(
        unix,
        not(any(target_os = "macos", target_os = "ios", target_os = "android"))
    ))]
    let store = zbus_secret_service_keyring_store::Store::new()?;
    #[cfg(all(any(unix, windows), not(any(target_os = "ios", target_os = "android"))))]
    {
        keyring::set_default_store(store);
        Ok(())
    }
    #[cfg(not(all(any(unix, windows), not(any(target_os = "ios", target_os = "android")))))]
    Err(Error::Invalid(
        "platform".to_owned(),
        "system keyring unavailable".to_owned(),
    ))
}

fn retry<T>(
    mut operation: impl FnMut() -> Result<T>,
    mut pause: impl FnMut(Duration),
) -> Result<T> {
    for delay in [100, 250] {
        match operation() {
            Err(Error::PlatformFailure(_)) => pause(Duration::from_millis(delay)),
            result => return result,
        }
    }
    operation()
}

pub fn read(key: &str) -> Result<String> {
    retry(|| entry(key)?.get_password(), std::thread::sleep)
}

pub fn write(key: &str, token: &str) -> Result<()> {
    retry(|| entry(key)?.set_password(token), std::thread::sleep)
}

pub fn delete(key: &str) -> Result<()> {
    retry(|| entry(key)?.delete_credential(), std::thread::sleep)
}

fn entry(key: &str) -> Result<Entry> {
    initialize()?;
    Entry::new("codejournal", key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn retries_transient_failures_and_keeps_the_original_error() {
        let mut attempts = 0;
        let result = retry(
            || {
                attempts += 1;
                if attempts < 3 {
                    Err(Error::PlatformFailure(Box::new(io::Error::other(
                        "bus unavailable",
                    ))))
                } else {
                    Ok("recovered")
                }
            },
            |_| {},
        );
        assert_eq!(result.unwrap(), "recovered");
        assert_eq!(attempts, 3);
        let error = retry::<()>(
            || {
                Err(Error::PlatformFailure(Box::new(io::Error::other(
                    "bus unavailable",
                ))))
            },
            |_| {},
        );
        assert!(error.unwrap_err().to_string().contains("bus unavailable"));
    }

    #[test]
    fn missing_credentials_are_not_retried() {
        let mut attempts = 0;
        let result = retry::<()>(
            || {
                attempts += 1;
                Err(Error::NoEntry)
            },
            |_| panic!("must not sleep"),
        );
        assert!(matches!(result, Err(Error::NoEntry)));
        assert_eq!(attempts, 1);
    }
}
