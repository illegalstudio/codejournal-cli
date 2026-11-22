//! A cached brief cannot say who is working now.
use serde_json::{Value, json};

/// How long cached sessions and delegations still describe the present.
const FRESH_MINUTES: i64 = 10;

/// Drops the live coordination sections of a cached brief unless it was saved moments ago.
pub fn withhold_live(result: &mut Value) {
    let saved = result["cached_at"]
        .as_str()
        .and_then(|at| chrono::DateTime::parse_from_rfc3339(at).ok());
    if saved.is_some_and(|at| {
        chrono::Utc::now().signed_duration_since(at).num_minutes() < FRESH_MINUTES
    }) {
        return;
    }
    let mut withheld = Vec::new();
    for name in ["other_sessions", "delegations"] {
        if result[name].as_array().is_some_and(|rows| !rows.is_empty()) {
            withheld.push(name);
            result[name] = json!([]);
        }
    }
    if !withheld.is_empty() {
        result["live_withheld"] = json!(withheld);
    }
}

pub fn notice(data: &Value) -> String {
    let mut text = format!(
        "Cached brief{}: metadata may be outdated, and listed documents may not have cached bodies. Do not replace missing cached rules with new rules during an outage.",
        data["cached_at"]
            .as_str()
            .map_or(String::new(), |at| format!(" from {at}"))
    );
    if data["live_withheld"].is_array() {
        text.push_str(" The other sessions and delegations it listed are withheld: they may have ended since. Read current delegations with `cj delegation list`.");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_brief_cached_moments_ago_keeps_its_live_sections() {
        let brief = |at: chrono::DateTime<chrono::Utc>| {
            json!({"cached_at": at.to_rfc3339(), "other_sessions": [{"agent": "codex"}],
                "delegations": [{"status": "running"}], "tasks": [{"id": "t"}]})
        };
        let mut fresh = brief(chrono::Utc::now());
        withhold_live(&mut fresh);
        assert_eq!(fresh["delegations"].as_array().unwrap().len(), 1);
        assert!(!notice(&fresh).contains("withheld"));
        let mut old = brief(chrono::Utc::now() - chrono::Duration::hours(3));
        withhold_live(&mut old);
        assert_eq!(old["delegations"], json!([]));
        assert_eq!(old["other_sessions"], json!([]));
        assert_eq!(old["tasks"].as_array().unwrap().len(), 1);
        assert!(notice(&old).contains("withheld") && notice(&old).contains("Cached brief from "));
        let mut undated = json!({"delegations": [{"status": "running"}]});
        withhold_live(&mut undated);
        assert_eq!(undated["live_withheld"], json!(["delegations"]));
    }
}
