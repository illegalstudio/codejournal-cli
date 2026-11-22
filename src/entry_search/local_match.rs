//! Whether a remembered entry satisfies the filters of a search, as far as local data can tell.
use serde_json::Value;

pub fn matches(entry: &Value, params: &[(&str, String)], phrase: bool, any: bool) -> bool {
    let param = |name: &str| {
        params
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.as_str())
    };
    let text = |name: &str| entry[name].as_str().unwrap_or("");
    let topics: Vec<&str> = entry["topics"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    if param("project").is_some_and(|slug| text("project_slug") != slug)
        || param("kind").is_some_and(|kind| text("kind") != kind)
        || param("topic").is_some_and(|topic| !topics.contains(&topic))
        || (param("status") != Some("all") && !matches!(text("status"), "" | "active"))
    {
        return false;
    }
    let Some(query) = param("q").map(str::to_lowercase) else {
        return true;
    };
    let haystack = format!(
        "{} {} {} {}",
        text("title"),
        text("body"),
        text("seen_excerpt"),
        topics.join(" ")
    )
    .to_lowercase();
    if phrase {
        return haystack.contains(query.trim());
    }
    let mut words = query.split_whitespace();
    if any {
        words.any(|word| haystack.contains(word))
    } else {
        words.all(|word| haystack.contains(word))
    }
}

#[cfg(test)]
mod tests {
    use super::matches;
    use serde_json::json;

    fn params(items: &[(&'static str, &str)]) -> Vec<(&'static str, String)> {
        items
            .iter()
            .map(|(key, value)| (*key, (*value).to_owned()))
            .collect()
    }

    #[test]
    fn remembered_entries_follow_the_search_filters() {
        let entry = json!({"title": "The cache needs a flush", "kind": "gotcha", "status": "active",
            "project_slug": "one", "topics": ["cache"], "seen_excerpt": "Run artisan cache:clear"});
        let found = |items: &[(&'static str, &str)], phrase, any| {
            matches(&entry, &params(items), phrase, any)
        };
        assert!(found(
            &[("q", "FLUSH artisan"), ("project", "one")],
            false,
            false
        ));
        assert!(!found(&[("q", "flush missing")], false, false));
        assert!(found(&[("q", "flush missing")], false, true));
        assert!(found(&[("q", "needs a flush")], true, false));
        assert!(!found(&[("q", "flush needs")], true, false));
        assert!(!found(&[("q", "flush"), ("project", "two")], false, false));
        assert!(!found(&[("kind", "howto")], false, false));
        assert!(found(&[("topic", "cache")], false, false));
        let old = json!({"title": "Old", "status": "superseded"});
        assert!(!matches(&old, &params(&[]), false, false));
        assert!(matches(&old, &params(&[("status", "all")]), false, false));
    }
}
