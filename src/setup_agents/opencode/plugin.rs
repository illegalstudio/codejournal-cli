use super::{discovery, managed::Managed, root};
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

pub const EVENTS: [&str; 9] = [
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "PermissionRequest",
    "Notification",
    "Stop",
    "PreCompact",
    "SessionEnd",
];

fn sources(binary: &Path) -> Result<BTreeMap<String, String>> {
    let loader = format!(
        "// Managed by Code Journal.\nimport {{ createPlugin }} from './code-journal/plugin.mjs';\nexport const CodeJournal = createPlugin({});\n",
        serde_json::to_string(binary)?
    );
    Ok(BTreeMap::from([
        ("plugins/code-journal.js".to_owned(), loader),
        (
            "plugins/code-journal/plugin.mjs".to_owned(),
            include_str!("../../../integrations/opencode/plugin.mjs").to_owned(),
        ),
        (
            "plugins/code-journal/transport.mjs".to_owned(),
            include_str!("../../../integrations/opencode/transport.mjs").to_owned(),
        ),
        (
            "plugins/code-journal/sessions.mjs".to_owned(),
            include_str!("../../../integrations/opencode/sessions.mjs").to_owned(),
        ),
        (
            "plugins/code-journal/tools.mjs".to_owned(),
            include_str!("../../../integrations/opencode/tools.mjs").to_owned(),
        ),
    ]))
}

pub fn apply(binary: &Path, uninstall: bool, dry: bool, status: bool) -> Result<Value> {
    let root = root()?;
    let mut conflicts = Vec::new();
    for base in [
        discovery::default_root()?,
        discovery::home()?.join(".opencode"),
        root.clone(),
    ] {
        for directory in ["plugins", "plugin"] {
            for file in ["code-journal.js", "code-journal.ts"] {
                let path = base.join(directory).join(file);
                if path.is_file()
                    && !discovery::same_path(&path, &root.join("plugins/code-journal.js"))
                    && !conflicts.contains(&path)
                {
                    conflicts.push(path);
                }
            }
        }
    }
    if !conflicts.is_empty() && !uninstall && !status {
        bail!(
            "Another Code Journal OpenCode plugin is already present; remove the duplicate before setup: {}",
            conflicts
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let managed = Managed::load(&root, "plugin")?;
    let desired = sources(binary)?;
    let mut report = managed.apply(&desired, uninstall, dry, status)?;
    let present = if status
        || (uninstall
            && report["modified"]
                .as_array()
                .is_some_and(|files| !files.is_empty()))
    {
        report["installed"] == true
    } else {
        !uninstall
    };
    report["agent"] = json!("opencode");
    report["conflicts"] = json!(conflicts);
    report["settings"] = json!(root.join("plugins/code-journal.js"));
    report["installed"] = if present { json!(EVENTS) } else { json!([]) };
    report["missing"] = if present { json!([]) } else { json!(EVENTS) };
    report["removed"] = if uninstall && report["changed"] == true {
        json!(EVENTS)
    } else {
        json!([])
    };
    report["dry_run"] = json!(dry);
    report["preview"] = json!({"files": desired.keys().collect::<Vec<_>>(), "events": EVENTS});
    if status && let Some(fields) = report.as_object_mut() {
        fields.extend(crate::hook_status::activity("opencode")?);
    }
    Ok(report)
}
