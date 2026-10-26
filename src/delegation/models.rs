use super::args::Target;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{path::PathBuf, process::Command};

pub fn run(target: Target) -> Result<Value> {
    if matches!(target, Target::Codex) {
        let root = std::env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .or_else(|| directories::BaseDirs::new().map(|d| d.home_dir().join(".codex")))
            .context("home directory unavailable")?;
        let catalog: Value =
            serde_json::from_slice(&std::fs::read(root.join("models_cache.json")).context(
                "Codex model cache is unavailable; start the installed Codex CLI to refresh it",
            )?)?;
        let models = catalog["models"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|m| m["visibility"] == "list")
            .cloned()
            .collect::<Vec<_>>();
        return Ok(json!({"target":"codex","source":"local model cache","models":models}));
    }
    super::providers::available(target.name())?;
    let args: &[&str] = match target {
        Target::Claude => &["--help"],
        Target::Cursor | Target::Grok => &["models"],
        Target::Kimi => &["provider", "catalog", "list", "kimi-for-coding"],
        Target::Pi => &["--list-models"],
        Target::Codex => unreachable!(),
    };
    let output = Command::new(super::providers::executable(target.name()))
        .args(args)
        .output()?;
    ensure!(
        output.status.success(),
        "model discovery failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut value = json!({"target":target.name(),"output":String::from_utf8_lossy(&output.stdout),
        "notice":if matches!(target,Target::Claude){"Claude exposes model forms in help, not an account catalog"}else{""}});
    if matches!(target, Target::Kimi) {
        value["configured_aliases"] = aliases()?;
    }
    Ok(value)
}

fn aliases() -> Result<Value> {
    let output = Command::new("kimi")
        .args(["provider", "list", "--json"])
        .output()?;
    ensure!(
        output.status.success(),
        "Kimi configured model discovery failed"
    );
    let catalog: Value = serde_json::from_slice(&output.stdout)?;
    let rows = catalog["models"].as_object().into_iter().flatten().map(|(alias, model)| {
        json!({"alias": alias, "display_name": model["display_name"], "provider": model["provider"], "model": model["model"]})
    }).collect::<Vec<_>>();
    Ok(json!(rows))
}
