use crate::git;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use toml_edit::DocumentMut;

pub fn detect() -> Vec<String> {
    let Some(root) = git::root() else {
        return Vec::new();
    };
    let mut names = BTreeSet::new();
    if let Ok(go) = fs::read_to_string(root.join("go.mod")) {
        if let Some(module) = go.lines().find_map(|line| line.strip_prefix("module ")) {
            add(&mut names, module.trim());
            if let Some(last) = module.split('/').rev().find(|part| !version(part)) {
                add(&mut names, last);
            }
        }
    }
    if let Some(cargo) = toml(&root.join("Cargo.toml")) {
        add_item(&mut names, item(&cargo, &["package", "name"]));
        if let Some(bins) = cargo
            .get("bin")
            .and_then(toml_edit::Item::as_array_of_tables)
        {
            for bin in bins.iter() {
                add_item(&mut names, bin.get("name"));
            }
        }
    }
    if let Some(package) = json(&root.join("package.json")) {
        if package["private"] != true {
            if let Some(name) = package["name"].as_str() {
                add(&mut names, name.rsplit('/').next().unwrap_or(name));
            }
        }
        if let Some(bins) = package["bin"].as_object() {
            for name in bins.keys() {
                add(&mut names, name);
            }
        }
    }
    if let Some(pyproject) = toml(&root.join("pyproject.toml")) {
        add_item(&mut names, item(&pyproject, &["project", "name"]));
        add_item(&mut names, item(&pyproject, &["tool", "poetry", "name"]));
        for path in [
            item(&pyproject, &["project", "scripts"]),
            item(&pyproject, &["tool", "poetry", "scripts"]),
        ] {
            if let Some(table) = path.and_then(toml_edit::Item::as_table) {
                for (name, _) in table.iter() {
                    add(&mut names, name);
                }
            }
        }
    }
    if let Some(composer) = json(&root.join("composer.json")) {
        if composer["type"] != "project" {
            if let Some(name) = composer["name"].as_str() {
                add(&mut names, name);
                add(&mut names, name.rsplit('/').next().unwrap_or(name));
            }
        }
    }
    names.into_iter().collect()
}

fn add(names: &mut BTreeSet<String>, name: &str) {
    let value = name.trim().to_lowercase();
    if value.len() >= 2 {
        names.insert(value);
    }
}

fn add_item(names: &mut BTreeSet<String>, item: Option<&toml_edit::Item>) {
    if let Some(value) = item.and_then(toml_edit::Item::as_str) {
        add(names, value);
    }
}

fn item<'a>(document: &'a DocumentMut, keys: &[&str]) -> Option<&'a toml_edit::Item> {
    keys[1..]
        .iter()
        .try_fold(document.get(keys[0])?, |item, key| item.get(key))
}

fn version(segment: &str) -> bool {
    segment
        .strip_prefix('v')
        .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit()))
}

fn json(path: &Path) -> Option<Value> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

fn toml(path: &Path) -> Option<DocumentMut> {
    fs::read_to_string(path).ok()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::version;

    #[test]
    fn go_version_suffix_is_not_a_published_name() {
        assert!(version("v2"));
        assert!(!version("version"));
    }
}
