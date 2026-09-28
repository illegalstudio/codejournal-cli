use anyhow::{Context, Result, bail};
use std::path::Path;

pub fn slug(explicit: Option<&str>) -> Result<String> {
    let value = match explicit {
        Some(value) => value.to_owned(),
        None => std::env::current_dir()?
            .file_name()
            .and_then(|part| part.to_str())
            .context("cannot determine project from current directory")?
            .to_owned(),
    };
    let normalized = value.to_ascii_lowercase().replace(['_', ' '], "-");
    if normalized.is_empty()
        || !normalized
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        bail!("invalid project slug: {value}");
    }
    Ok(normalized)
}

pub fn name(explicit: Option<&str>) -> Result<String> {
    match explicit {
        Some(value) => Ok(value.to_owned()),
        None => Ok(Path::new(&std::env::current_dir()?)
            .file_name()
            .and_then(|part| part.to_str())
            .context("cannot determine project name")?
            .to_owned()),
    }
}
