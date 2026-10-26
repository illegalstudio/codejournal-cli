use super::{Scope, state};
use crate::{api::Api, config::Config, request_age, request_outbox};
use anyhow::Result;
use std::{thread, time::Duration};

pub fn run(server: Option<&str>, origin: &str) -> Result<()> {
    if !super::enabled() {
        return Ok(());
    }
    let config = Config::load()?;
    let server = server.unwrap_or(&config.server);
    let api = Api::with_timeout(server, &config.token_for(server)?, Duration::from_secs(10))?;
    let scope = Scope::new(&api, &config.tenant);
    if scope.origin != origin {
        return Ok(());
    }
    let Some(lock) = state::try_lock(&scope)? else {
        return Ok(());
    };
    let mut saved = state::load(&scope)?;
    crate::watches::process::install_signals();
    loop {
        if scope.pending()? == 0 {
            saved.next_retry_at = None;
            saved.last_error = None;
            saved.blocked = false;
            state::save(&scope, &saved)?;
            drop(lock);
            // Recheck after releasing ownership so a concurrent enqueue cannot lose its wakeup.
            if scope.pending()? > 0 {
                super::wake(&api, &scope.tenant)?;
            }
            return Ok(());
        }
        if !wait(&scope, saved.next_retry_at.unwrap_or(0)) {
            return Ok(());
        }
        // A new invocation allows one retry after a permanent error was corrected.
        saved.last_attempt_at = Some(request_age::now());
        saved.blocked = false;
        state::save(&scope, &saved)?;
        let delivery = (|| {
            let requests =
                request_outbox::delivery::flush(&api, Some((&scope.tenant, &scope.origin)), 50)?;
            let hooks = if scope.pending_hooks()? > 0 {
                crate::outbox_flush::run(&api, &scope.tenant)?
            } else {
                0
            };
            Ok::<_, anyhow::Error>(requests + hooks)
        })();
        match delivery {
            Ok(count) => {
                if count > 0 {
                    saved.last_sync_at = Some(request_age::now());
                    saved.failures = 0;
                    saved.last_error = None;
                }
                saved.next_retry_at = Some(request_age::now().saturating_add(1));
            }
            Err(error) => {
                let delay = super::retry::retry_delay(&error, saved.failures, &scope.origin);
                saved.failures = saved.failures.saturating_add(1);
                saved.last_error = Some(
                    crate::secret_redaction::text(&format!("{error:#}"))
                        .0
                        .chars()
                        .take(2000)
                        .collect(),
                );
                saved.next_retry_at =
                    delay.map(|seconds| request_age::now().saturating_add(seconds));
                saved.blocked = delay.is_none();
            }
        }
        state::save(&scope, &saved)?;
        if saved.blocked {
            return Ok(());
        }
    }
}

fn wait(scope: &Scope, until: u64) -> bool {
    loop {
        if crate::watches::process::stopped() {
            return false;
        }
        let Ok(config) = Config::load() else {
            return false;
        };
        let Ok(token) = config.token_for(&scope.server) else {
            return false;
        };
        if crate::outbox::fingerprint(&scope.server, &config.tenant, &token) != scope.origin {
            return false;
        }
        let remaining = until.saturating_sub(request_age::now());
        if remaining == 0 {
            return true;
        }
        thread::sleep(Duration::from_secs(remaining.min(5)));
    }
}
