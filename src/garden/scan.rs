use super::scan_batches;
use crate::{api::Api, git};
use anyhow::{Result, bail};
use serde_json::{Value, json};

pub fn run(api: &Api, base: &str, preview: bool) -> Result<Value> {
    let endpoint = if preview { "preview" } else { "scan" };
    let url = format!("{base}/garden/review/{endpoint}");
    let root = git::root();
    let mut findings = Vec::new();
    let mut reviewed = 0;
    let mut deferred = 0;
    let mut page = 1;
    let mut result = loop {
        let data = api.get(&format!("{base}/garden?page={page}&review=1"))?;
        let next = data["next_page"].as_u64();
        if next.is_some_and(|next| next <= page) {
            bail!("invalid garden pagination");
        }
        let mut batches = match &root {
            Some(root) => scan_batches::page(&data, root)?,
            None => Vec::new(),
        };
        if next.is_none() {
            if batches.is_empty() {
                batches.push(json!({}));
            }
            if let Some(last) = batches.last_mut() {
                last["complete"] = json!(true);
            }
        }
        let mut last = Value::Null;
        for body in batches {
            last = api.post_noqueue(&url, &body)?;
            if preview {
                accumulate(&last, &mut findings, &mut reviewed, &mut deferred);
            }
        }
        match next {
            Some(next) => page = next,
            None => break last,
        }
    };
    if preview {
        findings.sort_by(|a, b| {
            b["priority"]
                .as_i64()
                .cmp(&a["priority"].as_i64())
                .then_with(|| a["id"].as_str().cmp(&b["id"].as_str()))
        });
        findings.dedup_by(|a, b| a["id"] == b["id"]);
        result["counts"] = json!({"pending": findings.len(), "reviewed": reviewed, "deferred": deferred,
            "last_scan": result["progress"]["last_scan"], "last_review": result["progress"]["last_review"]});
        result["findings"] = json!(findings);
        result.as_object_mut().map(|value| value.remove("progress"));
    }
    Ok(result)
}

fn accumulate(result: &Value, findings: &mut Vec<Value>, reviewed: &mut u64, deferred: &mut u64) {
    findings.extend(result["findings"].as_array().into_iter().flatten().cloned());
    *reviewed += result["reviewed"].as_u64().unwrap_or(0);
    *deferred += result["deferred"].as_u64().unwrap_or(0);
}
