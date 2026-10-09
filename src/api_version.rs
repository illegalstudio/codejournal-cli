//! Identifies this release to the server, which answers 426 when it requires a newer one.

use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use std::time::Duration;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const REVISION: &str = env!("CJ_BUILD_REVISION");
pub const DIRTY: bool = !env!("CJ_BUILD_SUFFIX").is_empty();
pub const DISPLAY: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CJ_BUILD_REVISION"),
    env!("CJ_BUILD_SUFFIX"),
    ")"
);
pub const HEADER: &str = "X-Cj-Version";

/// HTTP client that sends the release in `X-Cj-Version` and the user agent.
pub fn client(timeout: Duration) -> reqwest::Result<Client> {
    let mut headers = HeaderMap::new();
    headers.insert(HEADER, HeaderValue::from_static(VERSION));
    Client::builder()
        .timeout(timeout)
        .user_agent(user_agent())
        .default_headers(headers)
        .build()
}

pub fn user_agent() -> String {
    let (os, arch) = (std::env::consts::OS, std::env::consts::ARCH);
    format!("cj/{VERSION} ({os}; {arch})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_agent_names_the_release() {
        assert!(user_agent().starts_with(&format!("cj/{VERSION} (")));
    }
}
