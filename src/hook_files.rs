use crate::session_state;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub fn edited(payload: &Value, cwd: &Path, root: Option<&str>) -> Vec<String> {
    let tool = payload["tool_name"].as_str().unwrap_or("");
    let input = &payload["tool_input"];
    let mut paths = Vec::new();
    if ["apply_patch", "ApplyPatch"].contains(&tool) {
        let texts = if let Some(text) = input.as_str() {
            vec![text.to_owned()]
        } else {
            input
                .as_object()
                .map(|map| {
                    map.values()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default()
        };
        let pattern = regex::Regex::new(
            r"(?m)^\*\*\* (?:Add|Update|Delete) File: (.+)$|^\*\*\* Move to: (.+)$",
        )
        .unwrap();
        for text in texts {
            for captures in pattern.captures_iter(&text) {
                if let Some(path) = captures.get(1).or_else(|| captures.get(2)) {
                    paths.push(path.as_str().trim().to_owned());
                }
            }
        }
    } else if [
        "Edit",
        "Write",
        "MultiEdit",
        "NotebookEdit",
        "StrReplace",
        "Delete",
    ]
    .contains(&tool)
    {
        for key in [
            "file_path",
            "notebook_path",
            "path",
            "target_file",
            "filePath",
        ] {
            if let Some(path) = input[key].as_str() {
                paths.push(path.to_owned());
                break;
            }
        }
    }
    paths
        .into_iter()
        .map(|path| relative(&path, cwd, root))
        .collect::<Vec<_>>()
}

fn relative(path: &str, cwd: &Path, root: Option<&str>) -> String {
    let path = PathBuf::from(path);
    let absolute = if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    };
    root.and_then(|root| absolute.strip_prefix(root).ok())
        .unwrap_or(&absolute)
        .to_string_lossy()
        .to_string()
}

pub fn conflicts(session: &str, root: &str, files: &[String]) -> Vec<String> {
    let mut warnings = Vec::new();
    for other in session_state::others().unwrap_or_default() {
        if (other.session_id == session && other.agent == crate::attribution::agent(None))
            || other.ended_at.is_some()
            || other.root.as_deref() != Some(root)
        {
            continue;
        }
        for file in files {
            let Some(when) = other.files.get(file) else {
                continue;
            };
            let Ok(then) = chrono::DateTime::parse_from_rfc3339(when) else {
                continue;
            };
            let minutes = chrono::Utc::now().signed_duration_since(then).num_minutes();
            if (0..=15).contains(&minutes) {
                warnings.push(format!(
                    "{file} was also edited in this same checkout by {} ({minutes} min ago)",
                    if other.agent.is_empty() {
                        "another agent"
                    } else {
                        &other.agent
                    }
                ));
            }
        }
    }
    warnings
}
