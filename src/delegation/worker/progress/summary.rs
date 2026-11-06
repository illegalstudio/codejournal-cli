use serde_json::Value;

const TEXT_FIELDS: &[&str] = &["text", "message", "data", "result", "content"];
const TOOL_FIELDS: &[&str] = &["name", "tool_name", "toolName", "title", "status"];

pub fn collect(value: &Value, selected: &mut Vec<String>, depth: usize) {
    if depth > 10 || selected.len() >= 12 {
        return;
    }
    match value {
        Value::Object(map) => {
            let kind = map.get("type").and_then(Value::as_str).unwrap_or("");
            let delta = map
                .get("delta")
                .and_then(|delta| delta["type"].as_str())
                .unwrap_or("");
            if noise(kind) || noise(delta) {
                return;
            }
            let before = selected.len();
            for name in TOOL_FIELDS {
                if let Some(text) = map.get(*name).and_then(Value::as_str) {
                    push(selected, name, text);
                }
            }
            for name in TEXT_FIELDS {
                if let Some(child) = map.get(*name) {
                    field(child, name, selected, depth + 1);
                }
            }
            for (name, child) in map {
                if matches!(name.as_str(), "input" | "arguments" | "rawInput") {
                    arguments(child, selected, depth + 1);
                } else if !TEXT_FIELDS.contains(&name.as_str())
                    && !TOOL_FIELDS.contains(&name.as_str())
                    && !matches!(
                        name.as_str(),
                        "rawOutput"
                            | "stdout"
                            | "stderr"
                            | "bytes"
                            | "thinking"
                            | "thought"
                            | "signature"
                            | "usage"
                            | "modelUsage"
                    )
                {
                    collect(child, selected, depth + 1);
                }
            }
            if before == selected.len() && !kind.is_empty() {
                push(selected, "type", kind);
            }
        }
        Value::Array(items) => {
            for item in items
                .iter()
                .take(16)
                .filter(|item| item.is_object() || item.is_array())
            {
                collect(item, selected, depth + 1);
            }
        }
        _ => {}
    }
}

fn field(value: &Value, name: &str, selected: &mut Vec<String>, depth: usize) {
    if let Some(text) = value.as_str() {
        let trimmed = text.trim();
        if trimmed.len() > 16384 && trimmed.starts_with(['{', '[']) {
            push(selected, name, "[large nested provider content omitted]");
            return;
        }
        if trimmed.len() <= 16384
            && trimmed.starts_with(['{', '['])
            && let Ok(decoded) = serde_json::from_str::<Value>(trimmed)
        {
            collect(&decoded, selected, depth);
        } else {
            push(selected, name, text);
        }
    } else {
        collect(value, selected, depth);
    }
}

fn arguments(value: &Value, selected: &mut Vec<String>, depth: usize) {
    if depth > 10 || selected.len() >= 12 {
        return;
    }
    if let Some(text) = value.as_str().filter(|text| text.len() <= 16384) {
        if let Ok(decoded) = serde_json::from_str::<Value>(text) {
            arguments(&decoded, selected, depth + 1);
        }
        return;
    }
    for name in [
        "command",
        "cmd",
        "path",
        "file_path",
        "description",
        "query",
        "url",
    ] {
        if let Some(text) = value.get(name).and_then(Value::as_str) {
            push(selected, name, text);
        }
    }
}

fn push(selected: &mut Vec<String>, name: &str, text: &str) {
    if selected.len() < 12 && !text.trim().is_empty() {
        selected.push(format!(
            "{name}={}",
            text.chars()
                .take(400)
                .collect::<String>()
                .replace(['\n', '\r'], " ")
        ));
    }
}

fn noise(kind: &str) -> bool {
    matches!(
        kind,
        "thought"
            | "thinking"
            | "thinking_delta"
            | "signature_delta"
            | "input_json_delta"
            | "usage"
            | "usage_update"
    )
}
