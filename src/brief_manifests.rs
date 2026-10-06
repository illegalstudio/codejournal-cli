use crate::{git, manifest_file};
use regex::Regex;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const FILES: &[&str] = &[
    "go.mod",
    "package.json",
    "composer.json",
    "Cargo.toml",
    "pyproject.toml",
    "requirements.txt",
    "mise.toml",
    ".mise.toml",
    ".tool-versions",
    "Makefile",
    "justfile",
    "AGENTS.md",
    "CLAUDE.md",
    "CONTRIBUTING.md",
    "README.md",
];

pub fn fingerprints() -> Option<Value> {
    let root = git::root()?;
    let words = Regex::new(r"[\p{L}\p{N}_./-]{2,}").ok()?;
    let code = Regex::new(r"`[^`\n]+`").ok()?;
    let mut plain = BTreeSet::new();
    let mut complex = BTreeSet::new();
    let mut quoted = BTreeSet::new();
    for name in FILES {
        let path = root.join(name);
        let Some(bytes) = manifest_file::read(&path) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        for token in words.find_iter(&text) {
            add(token.as_str(), &mut plain, &mut complex);
        }
        for span in code.find_iter(&text) {
            for token in words.find_iter(span.as_str()) {
                let word = token.as_str().trim_matches(['.', '/', '-']).to_lowercase();
                if !word.contains(['.', '/']) {
                    quoted.insert(fingerprint(&word));
                }
            }
        }
    }
    Some(json!({"plain": plain, "complex": complex, "code": quoted}))
}

fn add(token: &str, plain: &mut BTreeSet<String>, complex: &mut BTreeSet<String>) {
    let word = token.trim_matches(['.', '/', '-']).to_lowercase();
    if word.len() < 2 {
        return;
    }
    if !word.contains(['.', '/']) {
        plain.insert(fingerprint(&word));
        return;
    }
    for candidate in [word.as_str(), word.strip_suffix(".git").unwrap_or(&word)] {
        complex.insert(fingerprint(candidate));
        for (index, character) in candidate.char_indices() {
            if character != '.' && character != '/' {
                continue;
            }
            let suffix = &candidate[index + 1..];
            if suffix.contains(['.', '/']) {
                complex.insert(fingerprint(suffix));
            }
            let prefix = &candidate[..index];
            if prefix.contains(['.', '/']) {
                complex.insert(fingerprint(prefix));
            }
        }
    }
}

fn fingerprint(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::{add, fingerprint};
    use std::collections::BTreeSet;

    #[test]
    fn repository_names_are_hashed_without_leaking_manifest_text() {
        let mut plain = BTreeSet::new();
        let mut complex = BTreeSet::new();
        add("github.com/acme/toolbox.git", &mut plain, &mut complex);
        assert!(complex.contains(&fingerprint("github.com/acme/toolbox")));
        assert!(complex.contains(&fingerprint("acme/toolbox")));
        assert!(!plain.contains(&fingerprint("toolbox")));
    }
}
