use crate::{
    api_status, api_version, api_write, output, project_bootstrap, request_outbox::PendingRequest,
    secret_redaction,
};
use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use serde_json::Value;
use std::time::Duration;

pub struct Api {
    pub(crate) client: Client,
    pub(crate) server: String,
    pub(crate) token: String,
    pub(crate) offline: bool,
    pub(crate) auto_project: bool,
}

impl Api {
    pub fn new(server: &str, token: &str) -> Result<Self> {
        Self::with_timeout(server, token, Duration::from_secs(20))
    }

    pub fn with_timeout(server: &str, token: &str, timeout: Duration) -> Result<Self> {
        let server = server.trim_end_matches('/').to_owned();
        let url = reqwest::Url::parse(&server).context("invalid server URL")?;
        if url.scheme() != "https" && !is_local_http(&url) {
            bail!("server must use HTTPS outside localhost");
        }
        if url.username() != ""
            || url.password().is_some()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
        {
            bail!("server URL must contain only a scheme, host, and optional port");
        }
        Ok(Self {
            client: api_version::client(timeout)?,
            server,
            token: token.to_owned(),
            offline: false,
            auto_project: false,
        })
    }

    pub fn set_offline(&mut self, offline: bool) {
        self.offline = offline;
    }

    pub fn set_auto_project(&mut self, enabled: bool) {
        self.auto_project = enabled;
    }

    pub fn server(&self) -> &str {
        &self.server
    }

    pub fn offline(&self) -> bool {
        self.offline
    }

    pub fn get(&self, path: &str) -> Result<Value> {
        crate::api_read::get(self, path)
    }

    pub fn post(&self, path: &str, body: &Value) -> Result<Value> {
        api_write::mutate(self, "POST", path, Some(body.clone()))
    }

    pub fn post_noqueue(&self, path: &str, body: &Value) -> Result<Value> {
        let path = project_bootstrap::read_path(self, path)?;
        let mut body = body.clone();
        output::record_masking(secret_redaction::value(&mut body));
        self.send(
            self.client
                .post(format!("{}{}", self.server, path))
                .json(&body),
        )
    }

    pub fn put_noqueue(&self, path: &str, body: &Value) -> Result<Value> {
        let mut body = body.clone();
        output::record_masking(secret_redaction::value(&mut body));
        self.send(
            self.client
                .put(format!("{}{}", self.server, path))
                .json(&body),
        )
    }

    pub fn delete(&self, path: &str) -> Result<Value> {
        api_write::mutate(self, "DELETE", path, None)
    }

    pub fn patch(&self, path: &str, body: &Value) -> Result<Value> {
        api_write::mutate(self, "PATCH", path, Some(body.clone()))
    }

    pub fn put(&self, path: &str, body: &Value) -> Result<Value> {
        api_write::mutate(self, "PUT", path, Some(body.clone()))
    }

    pub fn replay(&self, request: &PendingRequest) -> Result<Value> {
        api_write::replay(self, request)
    }

    fn send(&self, request: reqwest::blocking::RequestBuilder) -> Result<Value> {
        let response = request
            .bearer_auth(&self.token)
            .header("Accept", "application/json")
            .send()
            .context("API request failed")?;
        let status = response.status();
        let value: Value = sanitized(response.json().context("API returned invalid JSON")?);
        if !status.is_success() {
            return Err(api_status::error(status, &value));
        }
        Ok(value)
    }
}

pub(crate) fn sanitized(mut value: Value) -> Value {
    secret_redaction::value(&mut value);
    value
}

pub fn public_client(server: &str) -> Result<Client> {
    Api::new(server, "")?;
    Ok(api_version::client(Duration::from_secs(20))?)
}

fn is_local_http(url: &reqwest::Url) -> bool {
    url.scheme() == "http"
        && url
            .host_str()
            .is_some_and(|host| host == "localhost" || host == "127.0.0.1")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_plain_http_to_remote_server() {
        assert!(Api::new("http://example.com", "token").is_err());
        assert!(Api::new("https://example.com", "token").is_ok());
        assert!(Api::new("http://127.0.0.1:8080", "token").is_ok());
        assert!(Api::new("https://example.com@other.test", "token").is_err());
    }
}
