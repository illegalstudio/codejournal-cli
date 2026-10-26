use crate::api::Api;
use anyhow::{Result, bail};
use serde_json::{Value, json};

pub fn preview(mut result: Value, limit: usize, all: bool) -> Value {
    let mut rows = result["findings"].as_array().cloned().unwrap_or_default();
    if !all {
        rows.truncate(limit);
    }
    result["next"] = Value::Null;
    result["findings"] = json!(rows);
    if let Some(value) = result.as_object_mut() {
        value.remove("reviewed");
        value.remove("deferred");
    }
    result
}

pub fn complete(api: &Api, base: &str, result: &mut Value) -> Result<()> {
    let mut rows = result["findings"].as_array().cloned().unwrap_or_default();
    let mut next = result["next"].as_str().map(str::to_owned);
    let mut seen = std::collections::HashSet::new();
    while let Some(after) = next {
        if !seen.insert(after.clone()) {
            bail!("Invalid garden continuation");
        }
        let page = api.get(&format!("{base}/garden/review?limit=25&after={after}"))?;
        rows.extend(page["findings"].as_array().into_iter().flatten().cloned());
        next = page["next"].as_str().map(str::to_owned);
    }
    result["findings"] = json!(rows);
    result["next"] = Value::Null;
    Ok(())
}
