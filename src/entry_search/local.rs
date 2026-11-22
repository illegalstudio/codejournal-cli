//! Entries earlier searches saw on this machine, to answer a search the server cannot.
use crate::api::Api;
use crate::{api_cache, api_read::NoCachedResponse};
use anyhow::Result;
use serde_json::{Map, Value, json};

const LIMIT: usize = 500;

fn key(tenant: &str) -> String {
    format!("local-index:/api/v1/tenants/{tenant}/entries")
}

/// Reads from the server and remembers what it returned; without the server or an
/// exact cached copy, answers from the remembered entries.
pub fn get(api: &Api, tenant: &str, path: &str, params: &[(&str, String)]) -> Result<Value> {
    match api.get(path) {
        Ok(response) => {
            remember(api, tenant, param(params, "project"), &response);
            Ok(response)
        }
        Err(error) if error.downcast_ref::<NoCachedResponse>().is_some() => {
            answer(api, tenant, params).ok_or(error)
        }
        Err(error) => Err(error),
    }
}

fn param<'a>(params: &'a [(&str, String)], name: &str) -> Option<&'a str> {
    params
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value.as_str())
}

pub fn remember(api: &Api, tenant: &str, project: Option<&str>, response: &Value) {
    let entries = response["entries"].as_array();
    let Some(entries) = entries.filter(|items| !items.is_empty() && response["cached"] != true)
    else {
        return;
    };
    let mut seen = load(api, tenant);
    let now = crate::session_state::now();
    for entry in entries {
        let Some(id) = entry["id"].as_str() else {
            continue;
        };
        let mut saved = entry.clone();
        if let Some(fields) = saved.as_object_mut() {
            // The excerpt belongs to the query that found it; keep its words searchable only.
            let excerpt = fields.remove("match_excerpt").unwrap_or(Value::Null);
            fields.remove("match_quality");
            fields.insert("seen_excerpt".to_owned(), excerpt);
            fields.insert("seen_at".to_owned(), json!(now));
            if !fields.contains_key("project_slug") {
                fields.insert("project_slug".to_owned(), json!(project));
            }
            if fields.get("body").is_none_or(Value::is_null)
                && let Some(body) = seen.get(id).map(|old| old["body"].clone())
            {
                fields.insert("body".to_owned(), body);
            }
        }
        seen.insert(id.to_owned(), saved);
    }
    if seen.len() > LIMIT {
        let mut ages: Vec<_> = seen
            .iter()
            .map(|(id, entry)| {
                (
                    entry["seen_at"].as_str().unwrap_or("").to_owned(),
                    id.clone(),
                )
            })
            .collect();
        ages.sort();
        for (_, id) in ages.into_iter().take(seen.len() - LIMIT) {
            seen.remove(&id);
        }
    }
    let _ = api_cache::write(
        api.server(),
        &api.token,
        &key(tenant),
        &json!({"entries": seen}),
    );
}

fn load(api: &Api, tenant: &str) -> Map<String, Value> {
    api_cache::read(api.server(), &api.token, &key(tenant))
        .ok()
        .and_then(|index| index["entries"].as_object().cloned())
        .unwrap_or_default()
}

fn answer(api: &Api, tenant: &str, params: &[(&str, String)]) -> Option<Value> {
    // A path or exact reference filter needs the server's reference tables.
    if param(params, "path").is_some() || param(params, "ref_kind").is_some() {
        return None;
    }
    let seen = load(api, tenant);
    if seen.is_empty() {
        return None;
    }
    let flag = |name: &str| param(params, name) == Some("1");
    let mut entries: Vec<Value> = seen
        .values()
        .filter(|entry| {
            super::local_match::matches(
                entry,
                params,
                flag("literal") || flag("whole_word"),
                flag("any"),
            )
        })
        .cloned()
        .collect();
    entries.sort_by(|a, b| b["created_at"].as_str().cmp(&a["created_at"].as_str()));
    entries.truncate(
        param(params, "limit")
            .and_then(|limit| limit.parse().ok())
            .unwrap_or(20),
    );
    let seen_at = |pick: fn(&str, &str) -> bool| {
        seen.values()
            .filter_map(|entry| entry["seen_at"].as_str())
            .reduce(|kept, next| if pick(next, kept) { next } else { kept })
    };
    Some(json!({
        "entries": entries, "cached": true, "relaxed": false,
        "search_mode": if flag("whole_word") { "whole_word" } else { "local" },
        "local_index": {"indexed": seen.len(), "oldest_seen_at": seen_at(|next, kept| next < kept),
            "newest_seen_at": seen_at(|next, kept| next > kept)},
    }))
}
