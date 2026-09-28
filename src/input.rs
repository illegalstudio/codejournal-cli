use anyhow::{Context, Result, bail};
use std::io::{IsTerminal, Read};

pub fn body(explicit: Option<String>, file: Option<String>) -> Result<String> {
    if let Some(value) = explicit {
        return Ok(value);
    }
    if let Some(path) = file {
        return if path == "-" {
            read_stdin()
        } else {
            std::fs::read_to_string(&path).with_context(|| format!("cannot read {path}"))
        };
    }
    if !std::io::stdin().is_terminal() {
        return read_stdin();
    }
    bail!("body required: pass --body, --body-file PATH, or pipe it on stdin")
}

fn read_stdin() -> Result<String> {
    let mut value = String::new();
    std::io::stdin().read_to_string(&mut value)?;
    Ok(value)
}
