use anyhow::Result;
use serde_json::Value;

pub fn json(value: &Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
