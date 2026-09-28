use crate::api::Api;
use anyhow::{Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};

pub fn choose(api: &Api, tenant: &str) -> Result<Option<String>> {
    let result = api.get(&format!("/api/v1/tenants/{tenant}/projects"))?;
    let mut lines = vec!["*	All projects".to_owned()];
    for row in result["projects"].as_array().into_iter().flatten() {
        let slug = row["slug"].as_str().unwrap_or("");
        let name = row["name"]
            .as_str()
            .unwrap_or(slug)
            .replace(['\n', '\t'], " ");
        lines.push(format!("{slug}\t{name}\t{} active", row["active_entries"]));
    }
    if lines.len() == 1 {
        return Ok(Some("*".into()));
    }
    let mut process = Command::new("fzf")
        .args([
            "--ansi",
            "--delimiter",
            "\t",
            "--with-nth",
            "2..",
            "--no-sort",
            "--prompt",
            "project> ",
            "--header",
            "choose a project | enter: browse its entries | esc: quit",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("browse needs fzf on PATH")?;
    if let Some(mut stdin) = process.stdin.take() {
        stdin.write_all(lines.join("\n").as_bytes())?;
    }
    let output = process.wait_with_output()?;
    if !output.status.success() {
        return Ok(None);
    }
    let selected = String::from_utf8(output.stdout)?;
    Ok(selected.split('\t').next().map(str::to_owned))
}
