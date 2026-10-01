use std::path::Path;

pub fn command(binary: &Path) -> Option<&'static str> {
    let parts: Vec<_> = binary
        .components()
        .map(|part| part.as_os_str().to_string_lossy().to_lowercase())
        .collect();
    if parts.iter().any(|part| part == "cellar") {
        Some("brew upgrade illegalstudio/tap/codejournal-cli")
    } else if parts.iter().any(|part| part == "mise") && parts.iter().any(|part| part == "installs")
        || std::env::var_os("MISE_DATA_DIR")
            .is_some_and(|root| binary.starts_with(Path::new(&root).join("installs")))
    {
        Some("mise upgrade github:illegalstudio/codejournal-cli")
    } else if parts.iter().any(|part| part == "scoop") && parts.iter().any(|part| part == "apps") {
        Some("scoop update codejournal-cli")
    } else if binary
        .parent()
        .and_then(Path::parent)
        .is_some_and(|root| root.join(".crates.toml").exists())
    {
        Some("cargo install --git https://github.com/illegalstudio/codejournal-cli --locked")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_manager_binaries_are_never_directly_replaced() {
        assert!(
            command(Path::new(
                "/opt/homebrew/Cellar/codejournal-cli/0.1.0/bin/cj"
            ))
            .unwrap()
            .starts_with("brew")
        );
        assert!(command(Path::new("/home/example/.local/share/mise/installs/github-illegalstudio-codejournal-cli/0.1.0/cj")).unwrap().starts_with("mise"));
        assert!(command(Path::new("/home/example/.local/bin/cj")).is_none());
    }
}
