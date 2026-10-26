use crate::attribution;
use anyhow::{Context, Result, bail};
use std::path::PathBuf;

/// Host files use existing URL refs so they never become checkout-relative paths.
pub fn parse(raw: &str) -> Result<Option<String>> {
    let (host, path) = if let Some(path) = raw.strip_prefix("file:") {
        if path.starts_with("//") {
            return Ok(None);
        }
        (attribution::host(), expand(path)?)
    } else if let Some(value) = raw.strip_prefix("host:") {
        let (host, path) = value
            .split_once(':')
            .context("use host:HOST:/absolute/path")?;
        (host.to_owned(), path.to_owned())
    } else {
        return Ok(None);
    };
    if !path.starts_with('/')
        || host.is_empty()
        || !host.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'.' || byte == b'_'
        })
    {
        bail!("host file refs require a hostname and an absolute path");
    }
    let mut url = reqwest::Url::parse(&format!("file://{host}/"))?;
    url.set_path(&path);
    Ok(Some(url.to_string()))
}

fn expand(path: &str) -> Result<String> {
    if path.starts_with("~/") {
        let home = directories::BaseDirs::new().context("home directory unavailable")?;
        return Ok(home
            .home_dir()
            .join(&path[2..])
            .to_string_lossy()
            .into_owned());
    }
    Ok(PathBuf::from(path).to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_files_preserve_host_and_encode_paths() {
        assert_eq!(
            parse("host:example.com:/etc/my config").unwrap(),
            Some("file://example.com/etc/my%20config".into())
        );
        assert!(parse("file:relative.txt").is_err());
        assert!(parse("host:bad@host:/etc/config").is_err());
        assert_eq!(parse("path:/etc/config").unwrap(), None);
    }
}
