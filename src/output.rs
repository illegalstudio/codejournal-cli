use anyhow::Result;
use serde_json::Value;

pub fn json(value: &Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

pub fn emit(value: &Value, human: &str, json_mode: bool) -> Result<()> {
    if json_mode {
        json(value)
    } else {
        println!("{human}");
        Ok(())
    }
}
