use crate::secret_redaction;
use anyhow::Result;
use serde_json::Value;

pub fn json(value: &Value) -> Result<()> {
    let mut value = value.clone();
    secret_redaction::value(&mut value);
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

pub fn emit(value: &Value, human: &str, json_mode: bool) -> Result<()> {
    if json_mode {
        json(value)
    } else {
        let (clean, counts) = secret_redaction::text(human);
        println!("{clean}");
        if !counts.is_empty() {
            eprintln!(
                "cj: masked {} secret(s) in output",
                counts.values().sum::<usize>()
            );
        }
        Ok(())
    }
}
