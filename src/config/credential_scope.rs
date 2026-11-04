use super::Config;
use anyhow::{Result, bail, ensure};
use std::process::Command;

pub fn check(server: &str, stored_server: &str) -> Result<()> {
    if let Ok(issuer) = std::env::var("CJ_TOKEN_SERVER") {
        ensure!(
            server.trim_end_matches('/') == issuer.trim_end_matches('/'),
            "the delegated journal credential belongs to {issuer}; it cannot be sent to {server}"
        );
    }
    if std::env::var_os("CJ_TOKEN").is_none()
        && server.trim_end_matches('/') != stored_server.trim_end_matches('/')
    {
        bail!("the stored token belongs to {stored_server}; set CJ_TOKEN to use {server}");
    }
    Ok(())
}

/// Keep automatic credential transport bound to its issuer; explicit tokens retain their scope.
pub fn forward(command: &mut Command) {
    if std::env::var_os("CJ_TOKEN").is_some() {
        return;
    }
    let Ok(config) = Config::load() else {
        return;
    };
    let server = std::env::var("CJ_SERVER_URL").unwrap_or_else(|_| config.server.clone());
    if let Ok(token) = config.token_for(&server) {
        command
            .env("CJ_TOKEN", token)
            .env("CJ_TOKEN_SERVER", config.server);
    }
}
