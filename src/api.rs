use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use serde_json::Value;
use std::time::Duration;

pub struct Api {
    client: Client,
    server: String,
    token: String,
}

impl Api {
    pub fn new(server: &str, token: &str) -> Result<Self> {
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
            client: Client::builder().timeout(Duration::from_secs(20)).build()?,
            server,
            token: token.to_owned(),
        })
    }

    pub fn get(&self, path: &str) -> Result<Value> {
        self.send(self.client.get(format!("{}{}", self.server, path)))
    }

    pub fn post(&self, path: &str, body: &Value) -> Result<Value> {
        self.send(
            self.client
                .post(format!("{}{}", self.server, path))
                .json(body),
        )
    }

    pub fn delete(&self, path: &str) -> Result<Value> {
        self.send(self.client.delete(format!("{}{}", self.server, path)))
    }

    pub fn patch(&self, path: &str, body: &Value) -> Result<Value> {
        self.send(
            self.client
                .patch(format!("{}{}", self.server, path))
                .json(body),
        )
    }

    pub fn put(&self, path: &str, body: &Value) -> Result<Value> {
        self.send(
            self.client
                .put(format!("{}{}", self.server, path))
                .json(body),
        )
    }

    fn send(&self, request: reqwest::blocking::RequestBuilder) -> Result<Value> {
        let response = request
            .bearer_auth(&self.token)
            .header("Accept", "application/json")
            .send()
            .context("API request failed")?;
        let status = response.status();
        let value: Value = response.json().context("API returned invalid JSON")?;
        if !status.is_success() {
            bail!("API returned {status}: {value}");
        }
        Ok(value)
    }
}

pub fn public_client(server: &str) -> Result<Client> {
    Api::new(server, "")?;
    Ok(Client::builder().timeout(Duration::from_secs(20)).build()?)
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
