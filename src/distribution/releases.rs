use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::time::Duration;

const REPOSITORY: &str = "illegalstudio/codejournal-cli";

#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

pub fn client() -> Result<Client> {
    Ok(Client::builder()
        .https_only(true)
        .timeout(Duration::from_secs(120))
        .user_agent(concat!("cj/", env!("CARGO_PKG_VERSION")))
        .build()?)
}

pub fn latest(client: &Client) -> Result<Version> {
    let release: Release = client
        .get(format!(
            "https://api.github.com/repos/{REPOSITORY}/releases/latest"
        ))
        .send()?
        .error_for_status()?
        .json()?;
    stable_version(&release.tag_name)
}

fn stable_version(tag: &str) -> Result<Version> {
    let version = Version::parse(
        tag.strip_prefix('v')
            .context("release tag must start with v")?,
    )?;
    if !version.pre.is_empty() || !version.build.is_empty() {
        bail!("automatic updates require a stable release");
    }
    Ok(version)
}

pub fn platform() -> Result<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Ok("linux-x64"),
        ("linux", "aarch64") => Ok("linux-arm64"),
        ("macos", "x86_64") => Ok("macos-x64"),
        ("macos", "aarch64") => Ok("macos-arm64"),
        ("windows", "x86_64") => Ok("windows-x64"),
        _ => bail!("no published binary for this operating system and processor"),
    }
}

pub fn download(client: &Client, version: &Version) -> Result<Vec<u8>> {
    let extension = if cfg!(windows) { "zip" } else { "tar.gz" };
    let name = format!("codejournal-cli-v{version}-{}.{extension}", platform()?);
    let base = format!("https://github.com/{REPOSITORY}/releases/download/v{version}");
    let sums = String::from_utf8(fetch(client, &format!("{base}/SHA256SUMS"), 64 * 1024)?)?;
    let bytes = fetch(client, &format!("{base}/{name}"), 128 * 1024 * 1024)?;
    verify(&bytes, &sums, &name)?;
    Ok(bytes)
}

fn fetch(client: &Client, url: &str, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    client
        .get(url)
        .send()?
        .error_for_status()?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        bail!("release download exceeds its size limit");
    }
    Ok(bytes)
}

pub fn verify(bytes: &[u8], sums: &str, name: &str) -> Result<()> {
    let matches: Vec<_> = sums
        .lines()
        .filter_map(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            (fields.len() == 2 && fields[1] == name).then(|| fields[0])
        })
        .collect();
    if matches.len() != 1 || matches[0] != format!("{:x}", Sha256::digest(bytes)) {
        bail!("release checksum mismatch or ambiguous manifest; update aborted");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_stable_tags_can_form_download_urls() {
        assert_eq!(stable_version("v1.2.3").unwrap().to_string(), "1.2.3");
        for tag in ["1.2.3", "v1.2.3-rc.1", "v1.2.3+metadata", "v../secret"] {
            assert!(stable_version(tag).is_err());
        }
    }

    #[test]
    fn corrupt_missing_and_ambiguous_checksums_abort() {
        let body = b"binary";
        let line = format!("{:x}  cj.tar.gz\n", Sha256::digest(body));
        assert!(verify(body, &line, "cj.tar.gz").is_ok());
        assert!(verify(b"modified", &line, "cj.tar.gz").is_err());
        assert!(verify(body, &line, "other.tar.gz").is_err());
        assert!(verify(body, &line.repeat(2), "cj.tar.gz").is_err());
    }
}
