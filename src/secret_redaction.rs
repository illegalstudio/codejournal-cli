use regex::Regex;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::LazyLock;

static PATTERNS: LazyLock<Vec<(&str, Regex)>> = LazyLock::new(|| {
    [
        ("private key", r"-----BEGIN (?:[A-Z0-9]+ )*PRIVATE KEY-----[\s\S]*?-----END (?:[A-Z0-9]+ )*PRIVATE KEY-----"),
        ("github token", r"\b(?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9]{36,255}\b"),
        ("github token", r"\bgithub_pat_[A-Za-z0-9_]{22,255}\b"),
        ("gitlab token", r"\bglpat-[A-Za-z0-9_-]{20,}\b"),
        ("aws access key", r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b"),
        ("anthropic key", r"\bsk-ant-[A-Za-z0-9_-]{20,}"),
        ("openai key", r"\bsk-(?:proj-|svcacct-|admin-)?[A-Za-z0-9_-]{32,}"),
        ("stripe key", r"\b(?:sk|rk)_(?:live|test)_[A-Za-z0-9]{20,}\b"),
        ("slack token", r"\bxox[abposr]-[A-Za-z0-9-]{10,}\b"),
        ("slack webhook", r"https://hooks\.slack\.com/services/[A-Za-z0-9/]+"),
        ("google api key", r"\bAIza[0-9A-Za-z_-]{35}\b"),
        ("npm token", r"\bnpm_[A-Za-z0-9]{36}\b"),
        ("jwt", r"\beyJ[A-Za-z0-9_-]{8,}\.eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}"),
    ]
    .into_iter()
    .map(|(name, pattern)| (name, Regex::new(pattern).expect("valid secret pattern")))
    .collect()
});

static URL_PASSWORD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(\b[a-z][a-z0-9+.-]*://[^\s:/@]+:)([^\s@/]{3,})(@)")
        .expect("valid URL password pattern")
});

pub fn text(input: &str) -> (String, BTreeMap<String, usize>) {
    let mut clean = input.to_owned();
    let mut counts = BTreeMap::new();
    for (name, pattern) in PATTERNS.iter() {
        let count = pattern.find_iter(&clean).count();
        if count > 0 {
            clean = pattern
                .replace_all(&clean, format!("[redacted {name}]"))
                .into_owned();
            *counts.entry((*name).to_owned()).or_insert(0) += count;
        }
    }
    let count = URL_PASSWORD.find_iter(&clean).count();
    if count > 0 {
        clean = URL_PASSWORD
            .replace_all(&clean, "${1}[redacted password]${3}")
            .into_owned();
        counts.insert("url password".to_owned(), count);
    }
    (clean, counts)
}

pub fn value(value: &mut Value) -> BTreeMap<String, usize> {
    let mut found = BTreeMap::new();
    walk(value, &mut found);
    found
}

fn walk(value: &mut Value, found: &mut BTreeMap<String, usize>) {
    match value {
        Value::String(raw) => {
            let (clean, counts) = text(raw);
            *raw = clean;
            for (name, count) in counts {
                *found.entry(name).or_insert(0) += count;
            }
        }
        Value::Array(rows) => rows.iter_mut().for_each(|row| walk(row, found)),
        Value::Object(fields) => fields.values_mut().for_each(|row| walk(row, found)),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::text;

    #[test]
    fn masks_tokens_and_url_passwords_without_generic_false_positives() {
        let token = format!("ghp_{}", "A1b2".repeat(9));
        let (clean, found) = text(&format!("{token} https://ci:s3cretpw@example.test/r.git"));
        assert!(!clean.contains(&token));
        assert!(clean.contains("[redacted github token]"));
        assert!(clean.contains("ci:[redacted password]@"));
        assert_eq!(found["github token"], 1);
        assert_eq!(
            text("password=changeme; git@github.com:org/repo.git")
                .1
                .len(),
            0
        );
    }

    #[test]
    fn matches_python_secret_families() {
        let samples = [
            ("github token", format!("github_pat_{}", "a".repeat(22))),
            ("aws access key", format!("AKIA{}", "ABCDEFGHIJKLMNOP")),
            ("anthropic key", format!("sk-ant-api03-{}", "x".repeat(30))),
            ("openai key", format!("sk-proj-{}", "Ab1".repeat(12))),
            ("stripe key", format!("sk_live_{}", "a".repeat(24))),
            ("slack token", "xoxb-1234567890-abcdefghij".to_owned()),
            ("google api key", format!("AIza{}", "B".repeat(35))),
            ("npm token", format!("npm_{}", "c".repeat(36))),
            ("gitlab token", format!("glpat-{}", "d".repeat(20))),
        ];
        for (kind, secret) in samples {
            let (clean, found) = text(&format!("value {secret} end"));
            assert!(!clean.contains(&secret), "{kind}");
            assert_eq!(found[kind], 1, "{kind}");
        }
        let key = "-----BEGIN OPENSSH PRIVATE KEY-----\nabc\n-----END OPENSSH PRIVATE KEY-----";
        assert_eq!(text(key).0, "[redacted private key]");
    }
}
