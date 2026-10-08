use crate::{api::Api, config::Config, outbox, request_outbox};
use anyhow::Result;
use std::process::{Command, Stdio};

mod retry;
mod state;
pub(crate) mod status;
pub(crate) mod worker;

pub struct Scope {
    pub server: String,
    pub tenant: String,
    pub origin: String,
}

impl Scope {
    pub fn new(api: &Api, tenant: &str) -> Self {
        Self {
            server: api.server().to_owned(),
            tenant: tenant.to_owned(),
            origin: outbox::fingerprint(api.server(), tenant, &api.token),
        }
    }

    pub fn pending(&self) -> Result<usize> {
        Ok(self.pending_writes()? + self.pending_hooks()?)
    }

    pub fn pending_writes(&self) -> Result<usize> {
        Ok(request_outbox::entries()?
            .iter()
            .filter(|(_, request)| {
                request_outbox::delivery::matches(request, &self.server, &self.tenant, &self.origin)
            })
            .count())
    }

    pub fn pending_hooks(&self) -> Result<usize> {
        if std::env::var("CODE_JOURNAL_HOOK_FLUSH").as_deref() == Ok("off") {
            return Ok(0);
        }
        Ok(outbox::entries()?
            .iter()
            .filter(|(_, event)| event["origin"].as_str() == Some(&self.origin))
            .count())
    }
}

pub fn enabled() -> bool {
    std::env::var("CODE_JOURNAL_AUTO_SYNC").as_deref() != Ok("off")
}

pub fn wake(api: &Api, tenant: &str) -> Result<()> {
    if api.offline() || !enabled() {
        return Ok(());
    }
    let scope = Scope::new(api, tenant);
    if scope.pending()? == 0 {
        return Ok(());
    }
    let saved = state::load(&scope)?;
    if saved.blocked
        && saved
            .last_attempt_at
            .is_some_and(|time| crate::request_age::now().saturating_sub(time) < 60)
    {
        return Ok(());
    }
    let Some(lock) = state::try_lock(&scope)? else {
        return Ok(());
    };
    let mut process = Command::new(std::env::current_exe()?);
    process
        .args(["--server", api.server(), "request-sync", &scope.origin])
        .env_remove("CJ_PROJECT")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    crate::watches::process::detached(&mut process);
    // The worker takes ownership of the lock; redundant starters exit immediately.
    drop(lock);
    process.spawn()?;
    Ok(())
}

pub fn wake_configured() -> Result<()> {
    if !enabled() || (request_outbox::pending()? == 0 && outbox::pending()? == 0) {
        return Ok(());
    }
    let config = Config::load()?;
    let server = std::env::var("CJ_SERVER_URL").unwrap_or(config.server.clone());
    let api = Api::new(&server, &config.token_for(&server)?)?;
    wake(&api, &config.tenant)
}
