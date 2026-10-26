use super::{PendingRequest, directory, entries};
use crate::{api::Api, api_cache, project_cache, request_age, request_ids};
use anyhow::{Result, bail};
use fs2::FileExt;
use std::fs::{self, OpenOptions};

#[derive(Debug)]
pub struct Deferred(pub u64);

impl std::fmt::Display for Deferred {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "server retry delay remains in force until {}",
            self.0
        )
    }
}

impl std::error::Error for Deferred {}

pub fn matches(request: &PendingRequest, server: &str, tenant: &str, origin: &str) -> bool {
    request.server.trim_end_matches('/') == server.trim_end_matches('/')
        && request
            .path
            .starts_with(&format!("/api/v1/tenants/{tenant}/"))
        && request
            .origin
            .as_deref()
            .is_none_or(|value| value == origin)
}

pub fn flush(api: &Api, scope: Option<(&str, &str)>, limit: usize) -> Result<usize> {
    let lock = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(directory()?.join(".flush.lock"))?;
    if lock.try_lock_exclusive().is_err() {
        return Ok(0);
    }
    let mut count = 0;
    for (path, request) in entries()? {
        if request.server != api.server()
            || scope
                .is_some_and(|(tenant, origin)| !matches(&request, api.server(), tenant, origin))
        {
            continue;
        }
        if count == limit {
            break;
        }
        let result = replay(api, &request, scope.is_some());
        result.map_err(|error| {
            error.context(format!("queued request {} remains pending", request.id))
        })?;
        fs::remove_file(path)?;
        count += 1;
    }
    Ok(count)
}

fn replay(api: &Api, request: &PendingRequest, automatic: bool) -> Result<()> {
    if automatic && request.origin.is_none() {
        bail!(
            "this request has no verified credential scope; run cj sync under its original account"
        );
    }
    request_age::ensure_retryable(request.created_at)?;
    if let Some(time) = request.retry_at.filter(|time| *time > request_age::now()) {
        return Err(Deferred(time).into());
    }
    let mut replay = request.clone();
    let parts = request.path.split('/').collect::<Vec<_>>();
    if parts.len() > 6 && parts[5] == "projects" {
        let slug = project_cache::canonical(api, parts[4], parts[6])?;
        replay.path = crate::project_bootstrap::replace_slug(&replay.path, &slug);
    }
    request_ids::resolve(api, &mut replay)?;
    let response = api.replay(&replay)?;
    if let Some((old, new)) = project_cache::cache_created(api, request, &response)?
        && old != new
    {
        let tenant = request.path.split('/').nth(4).unwrap_or("");
        let old_slug = old.rsplit('/').next().unwrap_or("");
        let new_slug = new.rsplit('/').next().unwrap_or("");
        api_cache::write(
            api.server(),
            &api.token,
            &format!("/local-slug/{tenant}/{old_slug}"),
            &serde_json::json!({"slug": new_slug}),
        )?;
    }
    Ok(())
}
