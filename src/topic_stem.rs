use std::collections::HashSet;

pub fn normalize(raw: &str) -> String {
    let mut text = String::new();
    for ch in raw.trim().to_lowercase().chars() {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() || "._/-".contains(ch) {
            text.push(ch);
        } else if !text.ends_with('-') {
            text.push('-');
        }
    }
    text.trim_matches('-').to_owned()
}

pub fn forms(name: &str) -> HashSet<String> {
    let value = name.to_lowercase().replace(['-', '_', '.', '/'], "");
    let mut forms = HashSet::from([value.clone()]);
    if value.len() < 4 || value.ends_with("ss") {
        return forms;
    }
    if let Some(base) = value.strip_suffix("ies") {
        if value.len() > 4 {
            forms.insert(format!("{base}y"));
        }
    }
    for suffix in ["ing", "ed"] {
        if let Some(base) = value.strip_suffix(suffix) {
            if base.len() >= 3 {
                forms.insert(base.to_owned());
                forms.insert(format!("{base}e"));
            }
        }
    }
    if let Some(base) = value.strip_suffix("es") {
        if value.len() > 4 {
            forms.insert(base.to_owned());
        }
    }
    if let Some(base) = value.strip_suffix('s') {
        forms.insert(base.to_owned());
    }
    if forms.len() > 1 {
        forms.remove(&value);
    }
    forms
}

pub fn stem(name: &str) -> String {
    forms(name)
        .into_iter()
        .min_by_key(|form| (form.len(), form.clone()))
        .unwrap_or_default()
}
