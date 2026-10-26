use anyhow::Result;
use semver::Version;

pub fn matches(output: &str, version: &Version) -> Result<bool> {
    let pattern = format!(
        r"\Acj {}(?: \((?:[a-fA-F0-9]{{7,40}}|unknown)(?:\+dirty)?\))?\z",
        regex::escape(&version.to_string())
    );
    Ok(regex::Regex::new(&pattern)?.is_match(output))
}

#[cfg(test)]
mod tests {
    use super::matches;
    use semver::Version;

    #[test]
    fn build_diagnostics_preserve_exact_release_validation() {
        let version = Version::new(0, 1, 0);
        for output in [
            "cj 0.1.0",
            "cj 0.1.0 (abcdef012345)",
            "cj 0.1.0 (abcdef0+dirty)",
            "cj 0.1.0 (unknown)",
        ] {
            assert!(matches(output, &version).unwrap());
        }
        for output in [
            "cj 9.9.9 (abcdef0)",
            "cj 0x1x0",
            "cj 0.1.0 (invalid)",
            "cj 0.1.0\nextra output",
            "cj 0.1.0 (abcdef0) extra",
        ] {
            assert!(!matches(output, &version).unwrap());
        }
    }
}
