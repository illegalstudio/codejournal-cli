use crate::secret_redaction;
use anyhow::Result;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::BTreeMap;

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
        crate::stdout::println!("{clean}");
        Ok(())
    }
}
