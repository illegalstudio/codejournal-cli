use reqwest::{StatusCode, header::HeaderMap};

#[derive(Debug)]
pub struct ResponseError {
    pub status: StatusCode,
    pub retry_after: Option<u64>,
    message: String,
}

impl ResponseError {
    pub fn wrap(
        status: StatusCode,
        retry_after: Option<u64>,
        error: anyhow::Error,
    ) -> anyhow::Error {
        Self {
            status,
            retry_after,
            message: error.to_string(),
        }
        .into()
    }

    pub fn retryable(&self) -> bool {
        self.status.is_success()
            || self.status.is_server_error()
            || self.status == StatusCode::TOO_MANY_REQUESTS
            || self.status == StatusCode::REQUEST_TIMEOUT
    }
}

impl std::fmt::Display for ResponseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ResponseError {}

pub fn retry_after(headers: &HeaderMap) -> Option<u64> {
    let value = headers.get(reqwest::header::RETRY_AFTER)?.to_str().ok()?;
    if let Ok(seconds) = value.trim().parse() {
        return Some(seconds);
    }
    let date = chrono::DateTime::parse_from_rfc2822(value).ok()?;
    Some((date.timestamp() - crate::request_age::now() as i64).max(0) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_after_accepts_seconds_and_http_dates() {
        let mut headers = HeaderMap::new();
        headers.insert(reqwest::header::RETRY_AFTER, "120".parse().unwrap());
        assert_eq!(retry_after(&headers), Some(120));
        headers.insert(
            reqwest::header::RETRY_AFTER,
            "Thu, 01 Jan 1970 00:00:00 GMT".parse().unwrap(),
        );
        assert_eq!(retry_after(&headers), Some(0));
        headers.insert(reqwest::header::RETRY_AFTER, "invalid".parse().unwrap());
        assert_eq!(retry_after(&headers), None);
    }
}
