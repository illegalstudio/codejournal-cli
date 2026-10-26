use crate::request_outbox::{self, PendingRequest, QueuedWrite};
use anyhow::Result;

pub(crate) fn queued<T>(request: &PendingRequest) -> Result<T> {
    request_outbox::enqueue(request).map_err(|error| {
        error.context(format!(
            "write could not be queued (request {})",
            request.id
        ))
    })?;
    Err(QueuedWrite(request.id.clone()).into())
}

pub(crate) fn queued_after<T>(request: &PendingRequest, delay: Option<u64>) -> Result<T> {
    let mut request = request.clone();
    request.retry_at = delay.map(|seconds| {
        crate::request_age::now()
            .saturating_add(seconds)
            .saturating_add(1)
    });
    queued(&request)
}
