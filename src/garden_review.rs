use serde_json::{Value, json};

pub struct Review {
    pub duplicates: Vec<Value>,
    pub stale_docs: Vec<Value>,
    pub idle_plans: Vec<Value>,
    pub doc_candidates: Vec<Value>,
}

pub fn collect(records: &[Value], _entries: &[Value]) -> Review {
    let active: Vec<_> = records
        .iter()
        .filter(|record| record["type"] == "entry" && record["status"] == "active")
        .collect();
    let mut duplicates = Vec::new();
    for (i, a) in active.iter().enumerate() {
        let first = stems(a["title"].as_str().unwrap_or(""));
        if first.len() < 3 {
            continue;
        }
        for b in active.iter().skip(i + 1) {
            let second = stems(b["title"].as_str().unwrap_or(""));
            if second.len() < 3 {
                continue;
            }
            let union = first.union(&second).count();
            let overlap = first.intersection(&second).count();
            let score = overlap as f64 / union as f64;
            if score >= 0.6 {
                duplicates.push(
                    json!({"a": {"id": a["id"], "title": a["title"], "created_at": a["created_at"]},
                    "b": {"id": b["id"], "title": b["title"], "created_at": b["created_at"]},
                    "score": (score * 100.0).round() / 100.0}),
                );
            }
        }
    }
    duplicates.sort_by(|a, b| {
        b["score"]
            .as_f64()
            .partial_cmp(&a["score"].as_f64())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    duplicates.truncate(30);
    let cutoff = chrono::Utc::now() - chrono::Duration::days(14);
    let idle_plans = records.iter().filter(|record| record["type"] == "plan" && record["kind"] == "plan"
        && record["status"] == "active" && record["updated_at"].as_str()
            .and_then(|date| chrono::DateTime::parse_from_rfc3339(date).ok())
            .is_some_and(|date| date < cutoff)
        && !records.iter().any(|log| log["type"] == "log" && log["plan_id"] == record["id"]
            && log["created_at"].as_str().and_then(|date| chrono::DateTime::parse_from_rfc3339(date).ok())
                .is_some_and(|date| date >= cutoff)))
        .map(|record| json!({"id": record["id"], "title": record["title"], "updated_at": record["updated_at"]}))
        .collect();
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for entry in &active {
        for topic in entry["topics"].as_array().into_iter().flatten() {
            if let Some(name) = topic.as_str() {
                *counts.entry(name.to_owned()).or_default() += 1;
            }
        }
    }
    let mut counted = counts.into_iter().collect::<Vec<_>>();
    counted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let doc_candidates = counted
        .into_iter()
        .filter(|(_, count)| *count >= 8)
        .filter(|(name, _)| {
            !records.iter().any(|record| {
                record["type"] == "plan"
                    && record["kind"] == "doc"
                    && (record["status"] == "draft" || record["status"] == "current")
                    && crate::garden_doc_scope::covers(record, &active)
                    && ["title", "body"].iter().any(|field| {
                        record[*field].as_str().is_some_and(|text| {
                            text.to_lowercase().contains(&name.to_lowercase())
                                || stems(text).contains(&crate::topic_stem::stem(&name))
                        })
                    })
            })
        })
        .take(5)
        .map(|(topic, entries)| json!({"topic": topic, "entries": entries}))
        .collect();
    let stale_docs = records.iter().filter(|record| record["type"] == "plan" && record["kind"] == "doc"
        && (record["status"] == "draft" || record["status"] == "current"))
        .filter_map(|record| changed_code(record).filter(|count| *count > 0)
            .map(|code_changes| json!({"id": record["id"], "title": record["title"], "code_changes": code_changes})))
        .collect();
    Review {
        duplicates,
        stale_docs,
        idle_plans,
        doc_candidates,
    }
}

fn changed_code(doc: &Value) -> Option<u64> {
    crate::git::root()?;
    let refs = doc["refs"].as_array()?;
    let paths: Vec<_> = refs
        .iter()
        .filter(|item| item["kind"] == "path")
        .filter_map(|item| item["value"].as_str())
        .collect();
    if paths.is_empty() {
        return None;
    }
    let updated = doc["updated_at"].as_str()?;
    let after = chrono::DateTime::parse_from_rfc3339(updated).ok()? + chrono::Duration::seconds(1);
    let since = format!("--since={}", after.format("%Y-%m-%dT%H:%M:%SZ"));
    let mut args = vec!["rev-list", "--count", since.as_str(), "HEAD", "--"];
    args.extend(paths);
    crate::git::output(&args)?.parse().ok()
}

fn stems(title: &str) -> std::collections::BTreeSet<String> {
    const STOP: &str = " a an and are as at be by for from has have in into is it its not of on or that the this to was were when where which while with without no only than then there these those do does ";
    title
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(crate::topic_stem::stem)
        .filter(|word| !word.is_empty() && !STOP.contains(&format!(" {word} ")))
        .collect()
}
