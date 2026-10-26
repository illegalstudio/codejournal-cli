pub const ALL: [&str; 9] = [
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "Notification",
    "PermissionRequest",
    "Stop",
    "PreCompact",
    "SessionEnd",
];

pub fn spec(agent: &str, event: &str) -> Option<(Option<&'static str>, u64)> {
    match (agent, event) {
        (_, "SessionStart") => Some((Some("startup|resume|clear|compact"), 30)),
        (_, "UserPromptSubmit") => Some((None, 10)),
        ("claude", "PreToolUse") => Some((
            Some("Edit|Write|MultiEdit|NotebookEdit|StrReplace|Delete"),
            5,
        )),
        ("codex", "PreToolUse") | (_, "PostToolUse") => Some((Some("*"), 5)),
        ("claude", "Notification") => Some((
            Some(
                "permission_prompt|idle_prompt|elicitation_dialog|elicitation_url_dialog|agent_needs_input",
            ),
            10,
        )),
        ("codex", "PermissionRequest") => Some((Some("*"), 10)),
        (_, "Stop") => Some((None, 10)),
        (_, "PreCompact") => Some((None, 5)),
        ("claude", "SessionEnd") => Some((None, 5)),
        ("codex", "SessionEnd") => Some((None, 3)),
        _ => None,
    }
}
