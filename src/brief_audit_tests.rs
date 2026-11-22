use crate::brief_audit::{project, render};
use serde_json::json;

#[test]
fn legacy_payloads_keep_complete_rules_and_local_metadata_only() {
    let rules = "Required project rule.\n".repeat(1000);
    let body = "irrelevant body ".repeat(10000);
    let row = json!({"id": "work", "title": "Local work", "status": "active", "body": body, "search_vector": body});
    let full = json!({"rules": rules, "project": {"id": "p", "slug": "fixture", "rules": rules},
        "plans": [row], "docs": [row], "tasks": [row], "global_docs": [row], "recent": [row],
        "garden_hint": "Run maintenance", "other_sessions": [{"id": "s", "agent": "codex", "files": ["src/test.rs"]}]});
    let audit = project(&full);
    assert_eq!(audit["rules"], rules);
    assert_eq!(audit["tasks"][0]["title"], "Local work");
    assert_eq!(audit["other_sessions"][0]["files"][0], "src/test.rs");
    let serialized = audit.to_string();
    for hidden in [
        "body",
        "search_vector",
        "global_docs",
        "recent",
        "garden_hint",
    ] {
        assert!(!serialized.contains(&format!("\"{hidden}\"")));
    }
    assert!(serialized.len() < full.to_string().len() / 10);
    let text = render(&audit);
    assert!(text.contains(&rules));
    assert!(text.contains("Local work"));
}

#[test]
fn cached_and_locked_audits_preserve_access_notices() {
    let locked = project(
        &json!({"locked": true, "notices": ["Upgrade required"], "rules": "", "mode": "offline"}),
    );
    let text = render(&locked);
    assert!(text.contains("Cached audit"));
    assert!(text.contains("Upgrade required"));
    assert!(!text.contains("Project rules"));
}

#[test]
fn audit_distinguishes_detached_head_from_missing_git_state() {
    let audit = project(&json!({"other_sessions": [
        {"id": "a", "agent": "codex", "branch": "feat/worktree", "detached_head": false, "checkout_path": "/worktree"},
        {"id": "b", "agent": "claude", "branch": null, "detached_head": true, "checkout_path": "/detached"},
        {"id": "c", "agent": "grok", "checkout_path": "/legacy"}
    ]}));
    assert_eq!(audit["other_sessions"][1]["detached_head"], true);
    let text = render(&audit);
    assert!(text.contains("codex on feat/worktree at /worktree"));
    assert!(text.contains("claude (detached HEAD) at /detached"));
    assert!(text.contains("grok at /legacy"));
}
