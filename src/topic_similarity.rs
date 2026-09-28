use crate::topic_stem::{forms, stem};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

fn distance(a: &str, b: &str) -> usize {
    let mut previous: Vec<_> = (0..=b.len()).collect();
    for (i, ca) in a.bytes().enumerate() {
        let mut current = vec![i + 1];
        for (j, cb) in b.bytes().enumerate() {
            current.push(
                (previous[j + 1] + 1)
                    .min(current[j] + 1)
                    .min(previous[j] + usize::from(ca != cb)),
            );
        }
        previous = current;
    }
    previous[b.len()]
}

fn stripped_digits(value: &str) -> String {
    value.chars().filter(|ch| !ch.is_ascii_digit()).collect()
}

pub fn groups(rows: &Value) -> (Vec<Vec<String>>, Vec<Vec<String>>) {
    let mut counts = BTreeMap::new();
    for item in rows.as_array().into_iter().flatten() {
        if let Some(name) = item["name"].as_str() {
            counts.insert(name.to_owned(), item["count"].as_u64().unwrap_or(0));
        }
    }
    let names: Vec<_> = counts.keys().cloned().collect();
    let mut parent: Vec<_> = (0..names.len()).collect();
    let mut owners = HashMap::new();
    for (index, name) in names.iter().enumerate() {
        for form in forms(name) {
            if let Some(other) = owners.get(&form) {
                union(&mut parent, index, *other);
            } else {
                owners.insert(form, index);
            }
        }
    }
    let mut by_parent: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for (index, name) in names.iter().enumerate() {
        let root = find(&mut parent, index);
        by_parent.entry(root).or_default().push(name.clone());
    }
    let rank = |name: &String| {
        (
            std::cmp::Reverse(*counts.get(name).unwrap_or(&0)),
            name.len(),
            name.clone(),
        )
    };
    let mut certain = Vec::new();
    let mut leaders = Vec::new();
    for mut group in by_parent.into_values() {
        group.sort_by_key(&rank);
        leaders.push(group[0].clone());
        if group.len() > 1 {
            certain.push(group);
        }
    }
    leaders.sort_by_key(|name| stem(name));
    let mut possible = Vec::new();
    for (index, a) in leaders.iter().enumerate() {
        let sa = stem(a);
        for b in leaders.iter().skip(index + 1) {
            let sb = stem(b);
            if stripped_digits(&sa) == stripped_digits(&sb) {
                continue;
            }
            if sa.len().min(sb.len()) >= 6
                && sa.len().abs_diff(sb.len()) <= 2
                && distance(&sa, &sb) <= if sa.len() < 9 { 1 } else { 2 }
            {
                let mut pair = vec![a.clone(), b.clone()];
                pair.sort_by_key(&rank);
                possible.push(pair);
            }
        }
    }
    (certain, possible)
}

fn find(parent: &mut [usize], index: usize) -> usize {
    if parent[index] != index {
        parent[index] = find(parent, parent[index]);
    }
    parent[index]
}

fn union(parent: &mut [usize], a: usize, b: usize) {
    let first = find(parent, a);
    let second = find(parent, b);
    parent[first] = second;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merges_inflected_names_and_preserves_numbered_series() {
        let rows = json!([
            {"name": "release", "count": 3}, {"name": "releases", "count": 1},
            {"name": "plan01", "count": 1}, {"name": "plan02", "count": 1}
        ]);
        let (certain, possible) = groups(&rows);
        assert_eq!(certain, vec![vec!["release", "releases"]]);
        assert!(possible.is_empty());
    }
}
