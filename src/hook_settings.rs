use crate::hook_events;
use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const MARKER: &str = "CJ_RUST_HOOK=1";
pub fn read(path: &Path) -> Result<Value> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let value: Value = serde_json::from_slice(&fs::read(path)?)
        .with_context(|| format!("invalid JSON in {}", path.display()))?;
    if !value.is_object() {
        bail!("{} must contain a JSON object", path.display());
    }
    Ok(value)
}

pub fn installed(settings: &Value) -> Vec<&'static str> {
    hook_events::ALL
        .into_iter()
        .filter(|event| {
            settings["hooks"][event].as_array().is_some_and(|groups| {
                groups.iter().any(|group| {
                    group["hooks"]
                        .as_array()
                        .is_some_and(|hooks| hooks.iter().any(is_ours))
                })
            })
        })
        .collect()
}

pub fn owned_groups(settings: &Value) -> Value {
    let mut owned = Map::new();
    for event in hook_events::ALL {
        let groups = settings["hooks"][event]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|group| {
                group["hooks"]
                    .as_array()
                    .is_some_and(|items| items.iter().any(is_ours))
            })
            .cloned()
            .collect::<Vec<_>>();
        if !groups.is_empty() {
            owned.insert(event.to_owned(), json!(groups));
        }
    }
    json!({"hooks": owned})
}

pub fn update(settings: &mut Value, binary: &Path, agent: &str, install: bool) -> Result<()> {
    let hooks = settings
        .as_object_mut()
        .context("settings must be an object")?
        .entry("hooks")
        .or_insert_with(|| Value::Object(Map::new()));
    let groups = hooks.as_object_mut().context("hooks must be an object")?;
    for event in hook_events::ALL {
        let existing = groups.remove(event).unwrap_or_else(|| json!([]));
        let old = existing.as_array().context("hook event must be an array")?;
        let mut kept = Vec::new();
        for group in old {
            let Some(items) = group["hooks"].as_array() else {
                kept.push(group.clone());
                continue;
            };
            let rest: Vec<Value> = items
                .iter()
                .filter(|item| !is_ours(item))
                .cloned()
                .collect();
            if rest.is_empty() {
                continue;
            }
            let mut copy = group.clone();
            copy["hooks"] = json!(rest);
            kept.push(copy);
        }
        if install && let Some((matcher, timeout)) = hook_events::spec(agent, event) {
            let command = format!(
                "{MARKER} '{}' hook {event}",
                binary.to_string_lossy().replace('\'', "'\\''")
            );
            let mut group = json!({"hooks": [{
                "type": "command", "command": command, "timeout": timeout
            }]});
            if let Some(matcher) = matcher {
                group["matcher"] = json!(matcher);
            }
            kept.push(group);
        }
        if !kept.is_empty() {
            groups.insert(event.to_owned(), json!(kept));
        }
    }
    if groups.is_empty() {
        settings.as_object_mut().unwrap().remove("hooks");
    }
    Ok(())
}

pub fn write(path: &Path, settings: &Value) -> Result<Option<String>> {
    let parent = path.parent().context("invalid settings path")?;
    fs::create_dir_all(parent)?;
    let backup = if path.exists() {
        let epoch = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
        let backup = path.with_extension(format!("cj-backup-{epoch}.json"));
        fs::copy(path, &backup)?;
        Some(backup.display().to_string())
    } else {
        None
    };
    let temporary = path.with_extension(format!("cj-tmp-{}.json", std::process::id()));
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    file.write_all(serde_json::to_string_pretty(settings)?.as_bytes())?;
    file.write_all(b"\n")?;
    fs::rename(temporary, path)?;
    Ok(backup)
}

fn is_ours(item: &Value) -> bool {
    item["command"]
        .as_str()
        .is_some_and(|command| command.contains(MARKER))
}
