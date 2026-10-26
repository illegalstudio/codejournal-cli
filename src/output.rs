use crate::secret_redaction;
use anyhow::Result;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::BTreeMap;

pub(crate) mod discovery;

thread_local! {
    static MASKED: RefCell<BTreeMap<String, usize>> = RefCell::new(BTreeMap::new());
}

pub fn record_masking(counts: BTreeMap<String, usize>) {
    MASKED.with(|saved| {
        for (name, count) in counts {
            *saved.borrow_mut().entry(name).or_insert(0) += count;
        }
    });
}

pub fn masking_notice() -> Option<String> {
    MASKED.with(|saved| {
        let counts = std::mem::take(&mut *saved.borrow_mut());
        (!counts.is_empty()).then(|| {
            format!(
                "note: masked {} before saving; never paste secrets into Code Journal",
                counts
                    .into_iter()
                    .map(|(name, count)| format!("{count} {name}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
    })
}

pub fn json(value: &Value) -> Result<()> {
    let mut value = value.clone();
    secret_redaction::value(&mut value);
    crate::stdout::println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

pub fn emit(value: &Value, human: &str, json_mode: bool) -> Result<()> {
    if json_mode {
        json(value)
    } else {
        let (clean, _) = secret_redaction::text(human);
        if let Some(notice) = masking_notice() {
            crate::stdout::println!("{notice}");
        }
        crate::stdout::println!("{}", terminal(&clean));
        Ok(())
    }
}

fn terminal(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() && !matches!(character, '\n' | '\t') {
                character.escape_default().to_string()
            } else {
                character.to_string()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn terminal_controls_are_visible_without_changing_layout() {
        let clean = super::terminal("title\u{1b}]52;c;payload\u{7}\u{9b}2J\r\n\tend");
        assert!(!clean.contains(['\u{1b}', '\u{7}', '\u{9b}', '\r']));
        assert!(clean.contains("payload"));
        assert!(clean.contains("\n\tend"));
    }
}
