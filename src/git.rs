use std::path::PathBuf;
use std::process::Command;

pub fn output(args: &[&str]) -> Option<String> {
    let result = bytes(args)?;
    Some(String::from_utf8_lossy(&result).trim().to_owned())
}

pub fn bytes(args: &[&str]) -> Option<Vec<u8>> {
    let result = Command::new("git").args(args).output().ok()?;
    if !result.status.success() {
        return None;
    }
    Some(result.stdout)
}

pub fn root() -> Option<PathBuf> {
    output(&["rev-parse", "--show-toplevel"]).map(PathBuf::from)
}

pub fn forge_origin() -> Option<(String, String)> {
    let raw = output(&["remote", "get-url", "origin"])?;
    let (host, path) = if let Some((left, right)) = raw.split_once(':') {
        if left.starts_with("git@") {
            (left.trim_start_matches("git@").to_owned(), right.to_owned())
        } else {
            let url = reqwest::Url::parse(&raw).ok()?;
            (
                url.host_str()?.to_owned(),
                url.path().trim_start_matches('/').to_owned(),
            )
        }
    } else {
        let url = reqwest::Url::parse(&raw).ok()?;
        (
            url.host_str()?.to_owned(),
            url.path().trim_start_matches('/').to_owned(),
        )
    };
    if host != "github.com" && host != "gitlab.com" {
        return None;
    }
    let path = path
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .to_owned();
    if path.split('/').count() < 2 {
        return None;
    }
    Some((host, path))
}
