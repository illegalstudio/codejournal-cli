use crate::{api::response_error::ResponseError, request_age, request_outbox};

pub(super) fn retry_delay(error: &anyhow::Error, failures: u32, origin: &str) -> Option<u64> {
    if let Some(deferred) = error.downcast_ref::<request_outbox::delivery::Deferred>() {
        return Some(deferred.0.saturating_sub(request_age::now()));
    }
    let header = if let Some(response) = error.downcast_ref::<ResponseError>() {
        if !response.retryable() {
            return None;
        }
        response.retry_after
    } else if error.downcast_ref::<reqwest::Error>().is_some() {
        None
    } else {
        return None;
    };
    let backoff = 5_u64.saturating_mul(1_u64 << failures.min(6)).min(300);
    let jitter = origin.bytes().map(u64::from).sum::<u64>() % 4;
    Some(header.unwrap_or(0).max(backoff + jitter))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_back_off_and_honor_server_delay_without_retrying_rejections() {
        let error = ResponseError::wrap(
            reqwest::StatusCode::SERVICE_UNAVAILABLE,
            None,
            anyhow::anyhow!("unavailable"),
        );
        assert_eq!(retry_delay(&error, 0, "d"), Some(5));
        assert_eq!(retry_delay(&error, 2, "d"), Some(20));
        assert_eq!(retry_delay(&error, 100, "d"), Some(300));
        let limited = ResponseError::wrap(
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            Some(600),
            anyhow::anyhow!("limited"),
        );
        assert_eq!(retry_delay(&limited, 0, "d"), Some(600));
        let rejected = ResponseError::wrap(
            reqwest::StatusCode::FORBIDDEN,
            None,
            anyhow::anyhow!("forbidden"),
        );
        assert_eq!(retry_delay(&rejected, 0, "d"), None);
    }
}
